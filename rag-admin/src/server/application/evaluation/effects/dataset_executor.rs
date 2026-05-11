use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::generator::build_question_prompt;
use crate::server::application::evaluation::ports::EvaluationGenerator;
use crate::server::application::evaluation::question_filter::{
    GeneratedQuestionGate, QuestionFilterDecision,
};
use crate::server::application::evaluation::reference_locator::ReferenceLocator;
use crate::server::application::source_document::ports::BlobStore;
use crate::server::application::AppError;
use crate::server::domain::evaluation::dataset::commands::{
    AcceptQuestion, CompleteDatasetGeneration, EvaluationDatasetCommand, FailDatasetGeneration,
    RejectQuestion,
};
use crate::server::domain::evaluation::dataset::aggregate::EvaluationDataset;
use crate::server::domain::evaluation::question::EvaluationReference;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::event_sourcing::command_processor::CommandProcessor;
use crate::server::event_sourcing::process_manager::EffectExecutor;
use crate::shared::plain_f32_vec;

use crate::server::application::ports::Clock;

use super::dataset::{EvaluationDatasetEffect, GenerateDatasetEffect};

const ATTEMPT_MULTIPLIER: usize = 12;
const PREVIOUS_QUESTION_PROMPT_LIMIT: usize = 12;

pub struct EvaluationDatasetEffectExecutor {
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<dyn BlobStore>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    command_processor: Arc<CommandProcessor<EvaluationDataset>>,
    clock: Arc<dyn Clock>,
}

impl EvaluationDatasetEffectExecutor {
    pub fn new(
        source_document_repository: Arc<dyn SourceDocumentRepository>,
        blob_store: Arc<dyn BlobStore>,
        generator: Arc<dyn EvaluationGenerator>,
        embedding_service: Arc<EmbeddingService>,
        command_processor: Arc<CommandProcessor<EvaluationDataset>>,
        clock: Arc<dyn Clock>,
    ) -> Arc<Self> {
        Arc::new(Self {
            source_document_repository,
            blob_store,
            generator,
            embedding_service,
            command_processor,
            clock,
        })
    }

    async fn run_generation(&self, effect: &GenerateDatasetEffect) -> Result<(), AppError> {
        let doc = self
            .source_document_repository
            .load(effect.document_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("document {}", effect.document_id)))?;

        let bytes = self.blob_store.get(&doc.latest_content_hash).await?;
        let plain_text = String::from_utf8(bytes)
            .map_err(|e| AppError::Internal(format!("document content is not valid UTF-8: {e}")))?;

        let embedding_model = self
            .embedding_service
            .resolve(effect.embedding_model_id)
            .await?;
        let target = effect.target_question_count as usize;
        let max_attempts = (target * ATTEMPT_MULTIPLIER).max(target + 12);
        let excerpt_threshold = effect.excerpt_similarity_threshold_milli as f32 / 1000.0;
        let duplicate_threshold = effect.duplicate_similarity_threshold_milli as f32 / 1000.0;

        let mut gate = GeneratedQuestionGate::new(
            self.embedding_service.as_ref(),
            &embedding_model,
            excerpt_threshold,
            duplicate_threshold,
        );
        let mut previous_coverage: Vec<String> = Vec::new();
        let mut rejection_attempt: u32 = 0;
        let mut accepted_sequence: u32 = 0;

        info!(
            dataset_id = %effect.dataset_id,
            document_id = %effect.document_id,
            target,
            max_attempts,
            generation_model_id = %effect.generation_model_id,
            "starting dataset generation"
        );

        for attempt in 0..max_attempts {
            if gate.kept_count() >= target {
                break;
            }

            let prompt =
                build_question_prompt(&plain_text, recent_previous_coverage(&previous_coverage));
            let generated_result = self
                .generator
                .generate_question(effect.generation_model_id, prompt)
                .await;

            let generated = match generated_result {
                Ok(g) => g,
                Err(e) => {
                    warn!(
                        dataset_id = %effect.dataset_id,
                        attempt = attempt + 1,
                        error = %e,
                        "generation attempt failed"
                    );
                    continue;
                }
            };

            let shared_question =
                match ReferenceLocator::generated_to_question(&generated, &plain_text) {
                    Ok(q) => q,
                    Err(e) => {
                        debug!(
                            dataset_id = %effect.dataset_id,
                            attempt = attempt + 1,
                            error = %e,
                            "discarded generated question (reference locator)"
                        );
                        continue;
                    }
                };

            let decision = gate.try_accept(shared_question).await?;
            match decision {
                QuestionFilterDecision::Accepted { .. } => {
                    let q = gate
                        .latest_question()
                        .expect("gate.try_accept returned Accepted");
                    let references: Vec<EvaluationReference> = q
                        .references
                        .iter()
                        .map(|r| EvaluationReference {
                            content: r.content.clone(),
                            char_start: r.char_start,
                            char_end: r.char_end,
                            embedding: r.embedding.as_ref().map(|e| plain_f32_vec(e)),
                        })
                        .collect();

                    self.command_processor
                        .handle(
                            effect.dataset_id,
                            EvaluationDatasetCommand::AcceptQuestion(AcceptQuestion {
                                dataset_id: effect.dataset_id,
                                sequence: accepted_sequence,
                                question: q.question.clone(),
                                references,
                                embedding: None,
                                occurred_at: self.clock.now(),
                            }),
                        )
                        .await?;
                    info!(
                        dataset_id = %effect.dataset_id,
                        sequence = accepted_sequence,
                        kept = gate.kept_count(),
                        target,
                        "accepted question"
                    );
                    accepted_sequence += 1;
                    previous_coverage.push(question_coverage_entry(q));
                }
                QuestionFilterDecision::RejectedLowExcerptSimilarity { similarity } => {
                    rejection_attempt += 1;
                    debug!(
                        dataset_id = %effect.dataset_id,
                        attempt = rejection_attempt,
                        similarity = format!("{:.3}", similarity),
                        "rejected: low excerpt similarity"
                    );
                    self.command_processor
                        .handle(
                            effect.dataset_id,
                            EvaluationDatasetCommand::RejectQuestion(RejectQuestion {
                                dataset_id: effect.dataset_id,
                                attempt: rejection_attempt,
                                reason: format!("low excerpt similarity {:.1}%", similarity * 100.0),
                                occurred_at: self.clock.now(),
                            }),
                        )
                        .await?;
                }
                QuestionFilterDecision::RejectedDuplicate { similarity } => {
                    rejection_attempt += 1;
                    debug!(
                        dataset_id = %effect.dataset_id,
                        attempt = rejection_attempt,
                        similarity = format!("{:.3}", similarity),
                        "rejected: duplicate"
                    );
                    self.command_processor
                        .handle(
                            effect.dataset_id,
                            EvaluationDatasetCommand::RejectQuestion(RejectQuestion {
                                dataset_id: effect.dataset_id,
                                attempt: rejection_attempt,
                                reason: format!("duplicate similarity {:.1}%", similarity * 100.0),
                                occurred_at: self.clock.now(),
                            }),
                        )
                        .await?;
                }
            }
        }

        if gate.kept_count() == 0 {
            warn!(
                dataset_id = %effect.dataset_id,
                rejection_attempts = rejection_attempt,
                "dataset generation produced no usable questions"
            );
            self.command_processor
                .handle(
                    effect.dataset_id,
                    EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                        dataset_id: effect.dataset_id,
                        reason: "generator did not produce any usable questions".into(),
                        occurred_at: self.clock.now(),
                    }),
                )
                .await?;
            return Err(AppError::Upstream(
                "generator did not produce any usable evaluation questions".into(),
            ));
        }

        if gate.kept_count() < target {
            let reason = format!(
                "generator produced only {}/{} usable questions after {} attempts",
                gate.kept_count(),
                target,
                max_attempts
            );
            warn!(
                dataset_id = %effect.dataset_id,
                kept = gate.kept_count(),
                target,
                max_attempts,
                "dataset generation under-target"
            );
            self.command_processor
                .handle(
                    effect.dataset_id,
                    EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                        dataset_id: effect.dataset_id,
                        reason: reason.clone(),
                        occurred_at: self.clock.now(),
                    }),
                )
                .await?;
            return Err(AppError::Upstream(reason));
        }

        self.command_processor
            .handle(
                effect.dataset_id,
                EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                    dataset_id: effect.dataset_id,
                    occurred_at: self.clock.now(),
                }),
            )
            .await?;

        info!(
            dataset_id = %effect.dataset_id,
            accepted = gate.kept_count(),
            rejected = rejection_attempt,
            "dataset generation complete"
        );

        Ok(())
    }
}

#[async_trait]
impl EffectExecutor<EvaluationDatasetEffect> for EvaluationDatasetEffectExecutor {
    async fn execute(&self, effect: &EvaluationDatasetEffect) -> Result<(), AppError> {
        match effect {
            EvaluationDatasetEffect::GenerateDataset(e) => self.run_generation(e).await,
        }
    }
}

fn recent_previous_coverage(previous_coverage: &[String]) -> &[String] {
    let start = previous_coverage
        .len()
        .saturating_sub(PREVIOUS_QUESTION_PROMPT_LIMIT);
    &previous_coverage[start..]
}

fn question_coverage_entry(question: &crate::shared::EvaluationQuestionDto) -> String {
    let refs = question
        .references
        .iter()
        .take(2)
        .map(|r| truncate_str(&r.content, 160))
        .collect::<Vec<_>>()
        .join(" || ");
    format!(
        "Q: {} | Covered: {}",
        truncate_str(&question.question, 120),
        refs
    )
}

fn truncate_str(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let out = chars.by_ref().take(max).collect::<String>();
    if chars.next().is_some() {
        format!("{out}...")
    } else {
        out
    }
}
