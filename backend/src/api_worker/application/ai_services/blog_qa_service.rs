use std::collections::HashSet;
use std::fmt::Write;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use tracing::{info, warn};

use crate::api_worker::{
    application::{
        AiInferenceServiceTrait, AppError, CachedAnswer, ChargeOutcome, QaCacheServiceTrait,
        QaCoordinatorTrait, QueryFilter, Reference, ScoredChunk, SseEvent, TokenStream,
        VectorizeServiceTrait,
    },
    domain::{PostSlug, Question},
};

const SYSTEM_PROMPT: &str = include_str!("prompts/blog_qa_system_prompt.txt");

pub type AnswerStream = Pin<Box<dyn Stream<Item = SseEvent>>>;

#[async_trait(?Send)]
pub trait BlogQaServiceTrait {
    async fn answer_stream(
        &self,
        slug: &PostSlug,
        question: &str,
    ) -> Result<AnswerStream, AppError>;
}

pub struct BlogQaService {
    pub ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
    pub blog_vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
    pub glossary_vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
    pub cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
    pub coordinator: Arc<dyn QaCoordinatorTrait + Send + Sync>,
    pub generation_model: String,
    pub daily_cap: u32,
    pub blog_index_name: String,
    pub blog_top_k: u32,
    pub glossary_top_k: u32,
    pub min_score: f32,
}

impl BlogQaService {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
        blog_vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
        glossary_vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
        cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
        coordinator: Arc<dyn QaCoordinatorTrait + Send + Sync>,
        generation_model: String,
        daily_cap: u32,
        blog_index_name: String,
        blog_top_k: u32,
        glossary_top_k: u32,
        min_score: f32,
    ) -> Arc<Self> {
        Arc::new(Self {
            ai,
            blog_vectorize,
            glossary_vectorize,
            cache,
            coordinator,
            generation_model,
            daily_cap,
            blog_index_name,
            blog_top_k,
            glossary_top_k,
            min_score,
        })
    }

    async fn get_source_version(&self, slug_str: &str) -> Result<String, AppError> {
        self.cache
            .get_source_version(&self.blog_index_name, slug_str)
            .await?
            .ok_or_else(|| {
                warn!(
                    slug = slug_str,
                    index = self.blog_index_name.as_str(),
                    "no source_version in KV; ingest may not have run"
                );
                AppError::NotFound("This post is not indexed yet. Try again later.".to_string())
            })
    }
}

fn build_prompt(
    question: &str,
    blog: &[ScoredChunk],
    glossary: &[ScoredChunk],
) -> (String, String) {
    let system = SYSTEM_PROMPT.to_string();

    let mut user = String::from("Excerpts and glossary definitions:\n\n");

    for c in blog {
        let heading = if c.heading.is_empty() {
            "(intro)"
        } else {
            &c.heading
        };
        user.push_str("=== BLOG EXCERPT ===\n");
        _ = writeln!(user, "Title: {heading}");
        _ = write!(user, "Content:\n{}\n\n", c.text);
    }

    for c in glossary {
        let heading = if c.heading.is_empty() {
            "(untitled)"
        } else {
            &c.heading
        };
        user.push_str("=== GLOSSARY ENTRY ===\n");
        _ = writeln!(user, "Title: {heading}");
        _ = write!(user, "Content:\n{}\n\n", c.text);
    }

    _ = write!(user, "Question: {question}\n\nAnswer:");

    (system, user)
}

enum PreparedAnswer {
    Cached {
        answer: CachedAnswer,
    },
    Fallback {
        answer: String,
        model: String,
    },
    Generate {
        slug: String,
        source_version: String,
        hash: String,
        token_stream: TokenStream,
        references: Vec<Reference>,
        model: String,
        cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
    },
}

#[async_trait(?Send)]
impl BlogQaServiceTrait for BlogQaService {
    async fn answer_stream(
        &self,
        slug: &PostSlug,
        question: &str,
    ) -> Result<AnswerStream, AppError> {
        let question =
            Question::create(question).map_err(|e| AppError::ValidationError(e.to_string()))?;

        let hash = question.hash();
        let slug_str = slug.as_str();

        let source_version = self.get_source_version(slug_str).await?;

        let prepared =
            if let Some(cached) = self.cache.get(slug_str, &source_version, &hash).await? {
                info!(slug = slug_str, hash = hash.as_str(), "qa cache hit");
                _ = self.coordinator.record_hit(slug_str, &hash).await;
                PreparedAnswer::Cached { answer: cached }
            } else {
                match self.coordinator.charge(slug_str, self.daily_cap).await? {
                    ChargeOutcome::Ok => {}
                    ChargeOutcome::RateLimited { retry_after_ms } => {
                        let secs = retry_after_ms.div_ceil(1000).max(1);
                        return Err(AppError::RateLimited(format!(
                            "Too many questions for this post. Try again in ~{secs}s."
                        )));
                    }
                    ChargeOutcome::DailyCapExceeded => {
                        return Err(AppError::RateLimited(
                            "Daily question budget reached. Try again tomorrow.".to_string(),
                        ));
                    }
                }

                let embedding = self.ai.embed(question.as_str()).await?;

                let blog_filter = QueryFilter {
                    source_slug: slug_str,
                    source_version: source_version.as_str(),
                };
                let blog_matches = self
                    .blog_vectorize
                    .query(
                        &embedding,
                        Some(blog_filter),
                        self.blog_top_k,
                        self.min_score,
                    )
                    .await?;

                let glossary_matches = match self
                    .glossary_vectorize
                    .query(&embedding, None, self.glossary_top_k, self.min_score)
                    .await
                {
                    Ok(matches) => matches,
                    Err(e) => {
                        warn!(error = %e, "glossary vectorize query failed; continuing without it");
                        Vec::new()
                    }
                };

                if blog_matches.is_empty() {
                    warn!(slug = slug_str, "no relevant blog chunks");
                    PreparedAnswer::Fallback {
                        answer: "I don't see that in this post.".to_string(),
                        model: self.generation_model.clone(),
                    }
                } else {
                    let references = dedupe_references(&blog_matches, &glossary_matches);
                    let (system, user) =
                        build_prompt(question.as_str(), &blog_matches, &glossary_matches);
                    let token_stream = self.ai.generate_stream(&system, &user).await?;

                    PreparedAnswer::Generate {
                        slug: slug_str.to_string(),
                        source_version,
                        hash,
                        token_stream,
                        references,
                        model: self.generation_model.clone(),
                        cache: Arc::clone(&self.cache),
                    }
                }
            };

        let stream = stream! {
            match prepared {
                PreparedAnswer::Cached { answer } => {
                    yield SseEvent::Meta {
                        references: answer.references,
                        cached: true,
                        model: answer.model,
                    };
                    yield SseEvent::Delta { text: answer.answer };
                    yield SseEvent::Done;
                }
                PreparedAnswer::Fallback { answer, model } => {
                    yield SseEvent::Meta {
                        references: vec![],
                        cached: false,
                        model,
                    };
                    yield SseEvent::Delta { text: answer };
                    yield SseEvent::Done;
                }
                PreparedAnswer::Generate {
                    slug,
                    source_version,
                    hash,
                    mut token_stream,
                    references,
                    model,
                    cache,
                } => {
                    yield SseEvent::Meta {
                        references: references.clone(),
                        cached: false,
                        model: model.clone(),
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
                        references: references.clone(),
                        model,
                        ts: Utc::now().timestamp_millis(),
                    };
                    if let Err(e) = cache.put(&slug, &source_version, &hash, &cached_answer).await {
                        warn!(error = %e, "qa cache write failed");
                    }
                    yield SseEvent::Done;
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

fn dedupe_references(blog: &[ScoredChunk], glossary: &[ScoredChunk]) -> Vec<Reference> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for chunk in blog.iter().chain(glossary.iter()) {
        for reference in &chunk.references {
            if seen.insert(reference.url.clone()) {
                out.push(reference.clone());
            }
        }
    }
    out
}
