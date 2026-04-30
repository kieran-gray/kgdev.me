use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::api_worker::application::{
    AiInferenceServiceTrait, AppError, CachedAnswer, CachedSource, QaCacheServiceTrait,
    ScoredChunk, VectorizeServiceTrait,
};

const VECTORIZE_TOP_K: u32 = 10;
const RESULTS_PER_POST: usize = 4;
const MIN_SCORE: f32 = 0.55;
const MAX_QUESTION_CHARS: usize = 500;
const MIN_QUESTION_CHARS: usize = 3;

#[derive(Debug, Clone)]
pub struct AnswerResult {
    pub answer: String,
    pub sources: Vec<CachedSource>,
    pub model: String,
    pub cached: bool,
}

#[async_trait(?Send)]
pub trait BlogQaServiceTrait {
    async fn answer(&self, slug: &str, question: &str) -> Result<AnswerResult, AppError>;
}

pub struct BlogQaService {
    pub ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
    pub vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
    pub cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
    pub generation_model: String,
    pub daily_cap: u32,
}

impl BlogQaService {
    pub fn create(
        ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
        vectorize: Arc<dyn VectorizeServiceTrait + Send + Sync>,
        cache: Arc<dyn QaCacheServiceTrait + Send + Sync>,
        generation_model: String,
        daily_cap: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            ai,
            vectorize,
            cache,
            generation_model,
            daily_cap,
        })
    }
}

pub fn normalise_question(input: &str) -> String {
    let lowered = input.to_lowercase();
    let collapsed: String = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_string()
}

pub fn hash_question(normalised: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    let bytes = hasher.finalize();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn build_prompt(question: &str, chunks: &[ScoredChunk]) -> (String, String) {
    let system = concat!(
        "You answer questions about a single blog post. ",
        "Use ONLY the provided excerpts. ",
        "If the answer is not present, say \"I don't see that in this post.\" ",
        "Be concise. Quote briefly when it helps. Do not invent facts."
    )
    .to_string();

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
    async fn answer(&self, slug: &str, question: &str) -> Result<AnswerResult, AppError> {
        let trimmed = question.trim();
        if trimmed.len() < MIN_QUESTION_CHARS {
            return Err(AppError::ValidationError(
                "Question is too short".to_string(),
            ));
        }
        if trimmed.len() > MAX_QUESTION_CHARS {
            return Err(AppError::ValidationError(format!(
                "Question must be at most {MAX_QUESTION_CHARS} characters"
            )));
        }

        let normalised = normalise_question(trimmed);
        if normalised.len() < MIN_QUESTION_CHARS {
            return Err(AppError::ValidationError(
                "Question is too short".to_string(),
            ));
        }
        let hash = hash_question(&normalised);

        if let Some(cached) = self.cache.get(slug, &hash).await? {
            info!(slug, hash = hash.as_str(), "qa cache hit");
            return Ok(AnswerResult {
                answer: cached.answer,
                sources: cached.sources,
                model: cached.model,
                cached: true,
            });
        }

        let allowed = self
            .cache
            .check_and_increment_daily_cap(self.daily_cap)
            .await?;
        if !allowed {
            return Err(AppError::RateLimited(
                "Daily question budget reached. Try again tomorrow.".to_string(),
            ));
        }

        let embedding = self.ai.embed(&normalised).await?;

        let mut matches = self
            .vectorize
            .query(&embedding, slug, VECTORIZE_TOP_K)
            .await?;
        matches.retain(|m| m.score >= MIN_SCORE);
        matches.truncate(RESULTS_PER_POST);

        if matches.is_empty() {
            warn!(slug, "no relevant chunks");
            let answer = "I don't see that in this post.".to_string();
            let sources: Vec<CachedSource> = vec![];
            let cached_answer = CachedAnswer {
                answer: answer.clone(),
                sources: sources.clone(),
                model: self.generation_model.clone(),
                ts: now_ms(),
            };
            self.cache.put(slug, &hash, &cached_answer).await?;
            return Ok(AnswerResult {
                answer,
                sources,
                model: self.generation_model.clone(),
                cached: false,
            });
        }

        let (system, user) = build_prompt(&normalised, &matches);
        let answer = self.ai.generate(&system, &user).await?;

        let sources: Vec<CachedSource> = matches
            .iter()
            .map(|m| CachedSource {
                chunk_id: m.chunk_id,
                heading: m.heading.clone(),
                score: m.score,
            })
            .collect();

        let cached_answer = CachedAnswer {
            answer: answer.clone(),
            sources: sources.clone(),
            model: self.generation_model.clone(),
            ts: now_ms(),
        };
        if let Err(e) = self.cache.put(slug, &hash, &cached_answer).await {
            warn!(error = %e, "qa cache write failed");
        }

        Ok(AnswerResult {
            answer,
            sources,
            model: self.generation_model.clone(),
            cached: false,
        })
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_lowercases_trims_collapses() {
        assert_eq!(
            normalise_question("  How DOES Hibernation\twork? "),
            "how does hibernation work"
        );
    }

    #[test]
    fn normalise_strips_outer_punctuation() {
        assert_eq!(normalise_question("???what now???"), "what now");
    }

    #[test]
    fn hash_is_stable() {
        let a = hash_question("how does hibernation work");
        let b = hash_question("how does hibernation work");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn hash_is_sensitive_to_content() {
        assert_ne!(hash_question("hibernation"), hash_question("hibernate"));
    }
}
