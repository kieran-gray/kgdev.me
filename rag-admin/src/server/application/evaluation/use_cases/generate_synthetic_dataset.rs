use std::sync::Arc;

use tokio::sync::RwLock;

use crate::server::{
    application::{
        blog::ports::BlogSource,
        embedding::EmbeddingService,
        evaluation::{
            generator::build_question_prompt,
            question_filter::{GeneratedQuestionGate, QuestionFilterDecision},
            ports::{EvaluationDatasetStore, EvaluationGenerator},
            progress::EvaluationProgress,
            reference_locator::ReferenceLocator,
        },
        AppError,
    },
    domain::Post,
};
use crate::shared::{EvaluationDataset, EvaluationQuestion, SettingsDto};

const DATASET_GENERATION_ATTEMPT_MULTIPLIER: usize = 12;
const PREVIOUS_QUESTION_PROMPT_LIMIT: usize = 12;

pub struct GenerateSyntheticDatasetUseCase {
    blog_source: Arc<dyn BlogSource>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    settings: Arc<RwLock<SettingsDto>>,
    evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
}

impl GenerateSyntheticDatasetUseCase {
    pub fn new(
        blog_source: Arc<dyn BlogSource>,
        generator: Arc<dyn EvaluationGenerator>,
        embedding_service: Arc<EmbeddingService>,
        settings: Arc<RwLock<SettingsDto>>,
        evaluation_dataset_store: Arc<dyn EvaluationDatasetStore>,
    ) -> Self {
        Self {
            blog_source,
            generator,
            embedding_service,
            settings,
            evaluation_dataset_store,
        }
    }

    pub async fn execute(
        &self,
        slug: &str,
        progress: Arc<dyn EvaluationProgress>,
    ) -> Result<(), AppError> {
        progress
            .info(format!("fetching post {slug} for chunking evaluation..."))
            .await;

        let post = Post::try_new(self.blog_source.fetch(slug).await?)?;
        let settings = self.settings.read().await.clone();
        let evaluation_settings = settings.evaluation;
        let embedding_model = settings.embedding_model;
        let target_questions = evaluation_settings.question_count as usize;
        let max_attempts =
            (target_questions * DATASET_GENERATION_ATTEMPT_MULTIPLIER).max(target_questions + 12);

        progress
            .info(format!(
                "generating {target_questions} synthetic question(s) via {} ({}) with up to {max_attempts} attempts",
                evaluation_settings.generation_backend.as_str(),
                evaluation_settings.generation_model
            ))
            .await;

        let mut gate = GeneratedQuestionGate::new(
            self.embedding_service.as_ref(),
            &embedding_model,
            evaluation_settings.excerpt_similarity_threshold(),
            evaluation_settings.duplicate_similarity_threshold(),
        );
        let mut previous_coverage: Vec<String> = Vec::new();

        for attempt in 0..max_attempts {
            if gate.kept_count() >= target_questions {
                break;
            }

            let prompt = build_question_prompt(
                post.markdown_body(),
                recent_previous_coverage(&previous_coverage),
            );
            let generated = self
                .generator
                .generate_question(&evaluation_settings.generation_model, prompt)
                .await;

            match generated {
                Ok(generated) => {
                    match ReferenceLocator::generated_to_question(&generated, post.markdown_body())
                    {
                        Ok(question) => {
                            let decision = gate.try_accept(question).await?;
                            match decision {
                                QuestionFilterDecision::Accepted { kept } => {
                                    if let Some(question) = gate.latest_question() {
                                        previous_coverage.push(previous_coverage_entry(question));
                                    }
                                    progress
                                        .info(format!(
                                            "accepted evaluation question {kept}/{target_questions}"
                                        ))
                                        .await;
                                }
                                QuestionFilterDecision::RejectedLowExcerptSimilarity {
                                    similarity,
                                } => {
                                    progress
                                        .warn(format!(
                                            "discarded generated question: low excerpt similarity {:.1}%",
                                            similarity * 100.0
                                        ))
                                        .await;
                                }
                                QuestionFilterDecision::RejectedDuplicate { similarity } => {
                                    progress
                                        .warn(format!(
                                            "discarded generated question: duplicate similarity {:.1}%",
                                            similarity * 100.0
                                        ))
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            progress
                                .warn(format!("discarded generated question: {e}"))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    progress
                        .warn(format!("generation attempt {} failed: {e}", attempt + 1))
                        .await;
                }
            }
        }

        if gate.generated_count() == 0 {
            return Err(AppError::Upstream(
                "generator did not produce any usable evaluation questions".into(),
            ));
        }

        let stats = gate.stats();
        progress
            .info(format!(
                "filtered generated questions: kept {}/{}, low excerpt similarity {}, duplicates {}",
                gate.kept_count(),
                gate.generated_count(),
                stats.low_excerpt_similarity,
                stats.duplicate
            ))
            .await;

        if gate.kept_count() < target_questions {
            return Err(AppError::Upstream(format!(
                "generator produced only {}/{} usable evaluation questions after {} attempts",
                gate.kept_count(),
                target_questions,
                max_attempts
            )));
        }

        let questions = gate.into_questions(target_questions);

        let dataset = EvaluationDataset {
            slug: post.slug().to_string(),
            post_version: post.version().as_str().to_string(),
            generated_at: now_rfc3339(),
            embedding_model_backend: Some(embedding_model.backend),
            embedding_model_id: Some(embedding_model.id.clone()),
            embedding_model_dims: Some(embedding_model.dims),
            questions,
        };
        self.evaluation_dataset_store.store(&dataset).await?;

        progress
            .success(format!(
                "saved chunking evaluation dataset: {} question(s)",
                dataset.questions.len()
            ))
            .await;
        Ok(())
    }
}

fn recent_previous_coverage(previous_coverage: &[String]) -> &[String] {
    let start = previous_coverage
        .len()
        .saturating_sub(PREVIOUS_QUESTION_PROMPT_LIMIT);
    &previous_coverage[start..]
}

fn previous_coverage_entry(question: &EvaluationQuestion) -> String {
    let references = question
        .references
        .iter()
        .take(2)
        .map(|reference| truncate_for_prompt(&reference.content, 160))
        .collect::<Vec<_>>()
        .join(" || ");

    format!(
        "Q: {} | Covered: {}",
        truncate_for_prompt(&question.question, 120),
        references
    )
}

fn truncate_for_prompt(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::EvaluationReference;

    #[test]
    fn previous_coverage_entry_includes_question_and_reference_summary() {
        let question = EvaluationQuestion {
            question: "What increments the in-memory total?".into(),
            references: vec![EvaluationReference {
                content: "When the fetch handler receives a WebSocket upgrade request, it increments the in-memory total.".into(),
                char_start: 0,
                char_end: 0,
                embedding: None,
            }],
            embedding: None,
        };

        let entry = previous_coverage_entry(&question);

        assert!(entry.contains("Q: What increments the in-memory total?"));
        assert!(
            entry.contains("Covered: When the fetch handler receives a WebSocket upgrade request")
        );
    }

    #[test]
    fn recent_previous_coverage_returns_only_latest_entries() {
        let values = (0..20).map(|idx| format!("item-{idx}")).collect::<Vec<_>>();

        let recent = recent_previous_coverage(&values);

        assert_eq!(recent.len(), PREVIOUS_QUESTION_PROMPT_LIMIT);
        assert_eq!(recent.first().unwrap(), "item-8");
        assert_eq!(recent.last().unwrap(), "item-19");
    }
}
