use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use tracing::{info, warn};

use crate::api_worker::{
    application::{
        AiInferenceServiceTrait, AppError, CachedAnswer, CachedSource, ChargeOutcome,
        QaCacheServiceTrait, QaCoordinatorTrait, ScoredChunk, SseEvent, VectorizeServiceTrait,
    },
    domain::Question,
};

const VECTORIZE_TOP_K: u32 = 10;
const RESULTS_PER_POST: usize = 4;
const MIN_SCORE: f32 = 0.65;
const SYSTEM_PROMPT: &str = concat!(
    "You answer questions about a single blog post. Use ONLY the provided excerpts. ",
    "If the answer is not present, say \"I don't see that in this post.\" ",
    "Be concise. Quote briefly when it helps. Do not invent facts."
);

pub type AnswerStream = Pin<Box<dyn Stream<Item = SseEvent>>>;

#[async_trait(?Send)]
pub trait BlogQaServiceTrait {
    async fn answer_stream(&self, slug: &str, question: &str) -> Result<AnswerStream, AppError>;
}

pub struct BlogQaService {
    pub ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
    pub vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
    pub cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
    pub coordinator: Arc<dyn QaCoordinatorTrait + Send + Sync>,
    pub generation_model: String,
    pub daily_cap: u32,
}

impl BlogQaService {
    pub fn create(
        ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
        vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
        cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
        coordinator: Arc<dyn QaCoordinatorTrait + Send + Sync>,
        generation_model: String,
        daily_cap: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            ai,
            vectorize,
            cache,
            coordinator,
            generation_model,
            daily_cap,
        })
    }
}

fn build_prompt(question: &str, chunks: &[ScoredChunk]) -> (String, String) {
    let system = SYSTEM_PROMPT.to_string();

    let mut user = String::from("Excerpts from the post:\n\n");
    for (i, c) in chunks.iter().enumerate() {
        let heading = if c.heading.is_empty() {
            "(intro)"
        } else {
            c.heading.as_str()
        };
        user.push_str(&format!("[{}] {}\n{}\n\n", i + 1, heading, c.text));
    }
    user.push_str(&format!("Question: {question}\n\nAnswer:"));
    (system, user)
}

#[async_trait(?Send)]
impl BlogQaServiceTrait for BlogQaService {
    async fn answer_stream(&self, slug: &str, question: &str) -> Result<AnswerStream, AppError> {
        let question =
            Question::create(question).map_err(|e| AppError::ValidationError(e.to_string()))?;

        let hash = question.hash();

        let post_version = self.cache.get_post_version(slug).await?.ok_or_else(|| {
            warn!(slug, "no post_version in KV; ingest may not have run");
            AppError::InternalError("This post is not indexed yet. Try again later.".to_string())
        })?;

        let daily_cap = self.daily_cap;
        let slug = slug.to_string();
        let ai = Arc::clone(&self.ai);
        let vectorize = Arc::clone(&self.vectorize);
        let cache = Arc::clone(&self.cache);
        let coordinator = Arc::clone(&self.coordinator);
        let generation_model = self.generation_model.clone();

        let s = stream! {
            if let Some(cached) = match cache.get(&slug, &post_version, &hash).await {
                Ok(c) => c,
                Err(e) => {
                    yield SseEvent::Error { message: e.to_string() };
                    return;
                }
            } {
                info!(slug = slug.as_str(), hash = hash.as_str(), "qa cache hit");
                let _ = coordinator.record_hit(&slug, &hash).await;
                yield SseEvent::Meta {
                    sources: cached.sources,
                    cached: true,
                    model: cached.model,
                };
                yield SseEvent::Delta { text: cached.answer };
                yield SseEvent::Done;
                return;
            }

            match coordinator.charge(&slug, daily_cap).await {
                Ok(ChargeOutcome::Ok) => {}
                Ok(ChargeOutcome::RateLimited { retry_after_ms }) => {
                    let secs = retry_after_ms.div_ceil(1000).max(1);
                    yield SseEvent::Error {
                        message: format!("Too many questions for this post. Try again in ~{secs}s."),
                    };
                    return;
                }
                Ok(ChargeOutcome::DailyCapExceeded) => {
                    yield SseEvent::Error {
                        message: "Daily question budget reached. Try again tomorrow.".to_string(),
                    };
                    return;
                }
                Err(e) => {
                    yield SseEvent::Error { message: e.to_string() };
                    return;
                }
            }

            let embedding = match cache.get_embedding(&hash).await {
                Ok(Some(e)) => {
                    info!(hash = hash.as_str(), "embedding cache hit");
                    e
                }
                Ok(None) => match ai.embed(question.as_str()).await {
                    Ok(fresh) => {
                        if let Err(e) = cache.put_embedding(&hash, &fresh).await {
                            warn!(error = %e, "embedding cache write failed");
                        }
                        fresh
                    }
                    Err(e) => {
                        yield SseEvent::Error { message: e.to_string() };
                        return;
                    }
                },
                Err(e) => {
                    yield SseEvent::Error { message: e.to_string() };
                    return;
                }
            };

            let mut matches = match vectorize
                .query(&embedding, &slug, &post_version, VECTORIZE_TOP_K)
                .await
            {
                Ok(m) => m,
                Err(e) => {
                    yield SseEvent::Error { message: e.to_string() };
                    return;
                }
            };
            matches.retain(|m| m.score >= MIN_SCORE);
            matches.truncate(RESULTS_PER_POST);

            let sources: Vec<CachedSource> = matches
                .iter()
                .map(|m| CachedSource {
                    chunk_id: m.chunk_id,
                    heading: m.heading.clone(),
                    score: m.score,
                })
                .collect();

            if matches.is_empty() {
                warn!(slug = slug.as_str(), "no relevant chunks");
                let answer = "I don't see that in this post.".to_string();
                let cached_answer = CachedAnswer {
                    answer: answer.clone(),
                    sources: sources.clone(),
                    model: generation_model.clone(),
                    ts: now_ms(),
                };
                if let Err(e) = cache.put(&slug, &post_version, &hash, &cached_answer).await {
                    warn!(error = %e, "qa cache write failed");
                }
                yield SseEvent::Meta {
                    sources,
                    cached: false,
                    model: generation_model.clone(),
                };
                yield SseEvent::Delta { text: answer };
                yield SseEvent::Done;
                return;
            }

            yield SseEvent::Meta {
                sources: sources.clone(),
                cached: false,
                model: generation_model.clone(),
            };

            let (system, user) = build_prompt(question.as_str(), &matches);
            let mut token_stream = match ai.generate_stream(&system, &user).await {
                Ok(s) => s,
                Err(e) => {
                    yield SseEvent::Error { message: e.to_string() };
                    return;
                }
            };

            let mut answer = String::new();
            let mut errored = false;
            while let Some(item) = token_stream.next().await {
                match item {
                    Ok(token) => {
                        answer.push_str(&token);
                        yield SseEvent::Delta { text: token };
                    }
                    Err(e) => {
                        errored = true;
                        yield SseEvent::Error { message: e.to_string() };
                        break;
                    }
                }
            }

            if errored {
                return;
            }

            let cached_answer = CachedAnswer {
                answer: answer.clone(),
                sources: sources.clone(),
                model: generation_model.clone(),
                ts: now_ms(),
            };
            if let Err(e) = cache.put(&slug, &post_version, &hash, &cached_answer).await {
                warn!(error = %e, "qa cache write failed");
            }
            yield SseEvent::Done;
        };

        Ok(Box::pin(s))
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
