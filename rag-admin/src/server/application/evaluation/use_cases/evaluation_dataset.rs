use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::{
    command_handlers::EvaluationDatasetCommandHandler,
    generator::build_question_prompt,
    ports::EvaluationGenerator,
    progress::EvaluationProgress,
    question_filter::{GeneratedQuestionGate, QuestionFilterDecision},
    reference_locator::ReferenceLocator,
};
use crate::server::application::ports::{Clock, IdGenerator};
use crate::server::application::source_document::ports::BlobStore;
use crate::server::application::AppError;
use crate::server::domain::evaluation::dataset::commands::{
    AcceptQuestion, CompleteDatasetGeneration, EvaluationDatasetCommand, FailDatasetGeneration,
    RejectQuestion, RequestDatasetGeneration,
};
use crate::server::domain::evaluation::question::EvaluationReference;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::shared::{plain_f32_vec, SettingsDto};

const ATTEMPT_MULTIPLIER: usize = 12;
const PREVIOUS_QUESTION_PROMPT_LIMIT: usize = 12;

pub struct GenerateEvaluationDatasetRequest {
    pub document_id: Uuid,
    pub label: String,
    pub embedding_model_id: Uuid,
    pub generation_model: String,
    pub generation_backend: String,
    pub target_question_count: u32,
    pub excerpt_similarity_threshold_milli: u32,
    pub duplicate_similarity_threshold_milli: u32,
}

pub struct GenerateSyntheticDatasetUseCase {
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<dyn BlobStore>,
    generator: Arc<dyn EvaluationGenerator>,
    embedding_service: Arc<EmbeddingService>,
    dataset_command_handler: Arc<EvaluationDatasetCommandHandler>,
    settings: Arc<RwLock<SettingsDto>>,
    clock: Arc<dyn Clock>,
    id_generator: Arc<dyn IdGenerator>,
}

impl GenerateSyntheticDatasetUseCase {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_document_repository: Arc<dyn SourceDocumentRepository>,
        blob_store: Arc<dyn BlobStore>,
        generator: Arc<dyn EvaluationGenerator>,
        embedding_service: Arc<EmbeddingService>,
        dataset_command_handler: Arc<EvaluationDatasetCommandHandler>,
        settings: Arc<RwLock<SettingsDto>>,
        clock: Arc<dyn Clock>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Arc<Self> {
        Arc::new(Self {
            source_document_repository,
            blob_store,
            generator,
            embedding_service,
            dataset_command_handler,
            settings,
            clock,
            id_generator,
        })
    }

    pub async fn execute(
        &self,
        request: GenerateEvaluationDatasetRequest,
        progress: Arc<dyn EvaluationProgress>,
    ) -> Result<Uuid, AppError> {
        let doc = self
            .source_document_repository
            .load(request.document_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("document {}", request.document_id)))?;

        let bytes = self.blob_store.get(&doc.latest_content_hash).await?;
        let plain_text = String::from_utf8(bytes)
            .map_err(|e| AppError::Internal(format!("document content is not valid UTF-8: {e}")))?;

        let settings = self.settings.read().await.clone();
        let embedding_model = settings.embedding_model;
        let target = request.target_question_count as usize;
        let max_attempts = (target * ATTEMPT_MULTIPLIER).max(target + 12);

        let dataset_id = self.id_generator.new_uuid();
        let occurred_at = self.clock.now();

        self.dataset_command_handler
            .handle(
                dataset_id,
                EvaluationDatasetCommand::RequestDatasetGeneration(RequestDatasetGeneration {
                    dataset_id,
                    document_id: request.document_id,
                    document_version: doc.latest_version_number,
                    content_hash: doc.latest_content_hash.to_string(),
                    label: request.label.clone(),
                    target_question_count: request.target_question_count,
                    generation_model: request.generation_model.clone(),
                    generation_backend: request.generation_backend.clone(),
                    excerpt_similarity_threshold_milli: request.excerpt_similarity_threshold_milli,
                    duplicate_similarity_threshold_milli: request
                        .duplicate_similarity_threshold_milli,
                    embedding_model_id: request.embedding_model_id,
                    occurred_at: occurred_at.clone(),
                }),
            )
            .await?;

        progress
            .info(format!(
                "generating {} synthetic question(s) via {} ({}) with up to {} attempts",
                target, request.generation_backend, request.generation_model, max_attempts
            ))
            .await;

        let excerpt_threshold = request.excerpt_similarity_threshold_milli as f32 / 1000.0;
        let duplicate_threshold = request.duplicate_similarity_threshold_milli as f32 / 1000.0;

        let mut gate = GeneratedQuestionGate::new(
            self.embedding_service.as_ref(),
            &embedding_model,
            excerpt_threshold,
            duplicate_threshold,
        );
        let mut previous_coverage: Vec<String> = Vec::new();
        let mut rejection_attempt: u32 = 0;
        let mut accepted_sequence: u32 = 0;

        for attempt in 0..max_attempts {
            if gate.kept_count() >= target {
                break;
            }

            let prompt =
                build_question_prompt(&plain_text, recent_previous_coverage(&previous_coverage));
            let generated = self
                .generator
                .generate_question(&request.generation_model, prompt)
                .await;

            match generated {
                Ok(generated) => {
                    match ReferenceLocator::generated_to_question(&generated, &plain_text) {
                        Ok(shared_question) => {
                            let decision = gate.try_accept(shared_question).await?;
                            match decision {
                                QuestionFilterDecision::Accepted { kept } => {
                                    if let Some(q) = gate.latest_question() {
                                        previous_coverage.push(question_coverage_entry(q));

                                        let references: Vec<EvaluationReference> = q
                                            .references
                                            .iter()
                                            .map(|r| EvaluationReference {
                                                content: r.content.clone(),
                                                char_start: r.char_start,
                                                char_end: r.char_end,
                                                embedding: r
                                                    .embedding
                                                    .as_ref()
                                                    .map(|e| plain_f32_vec(e)),
                                            })
                                            .collect();

                                        let question_to_save = crate::server::domain::evaluation::question::EvaluationQuestion {
                                            sequence: accepted_sequence,
                                            question: q.question.clone(),
                                            references: references.clone(),
                                            embedding: None,
                                        };

                                        self.dataset_command_handler
                                            .handle_accept_question(
                                                dataset_id,
                                                EvaluationDatasetCommand::AcceptQuestion(
                                                    AcceptQuestion {
                                                        sequence: accepted_sequence,
                                                        question: q.question.clone(),
                                                        references,
                                                        embedding: None,
                                                        occurred_at: self.clock.now(),
                                                    },
                                                ),
                                                question_to_save,
                                            )
                                            .await?;
                                        accepted_sequence += 1;
                                    }
                                    progress
                                        .info(format!(
                                            "accepted evaluation question {kept}/{}",
                                            target
                                        ))
                                        .await;
                                }
                                QuestionFilterDecision::RejectedLowExcerptSimilarity {
                                    similarity,
                                } => {
                                    rejection_attempt += 1;
                                    self.dataset_command_handler
                                        .handle(
                                            dataset_id,
                                            EvaluationDatasetCommand::RejectQuestion(
                                                RejectQuestion {
                                                    attempt: rejection_attempt,
                                                    reason: format!(
                                                        "low excerpt similarity {:.1}%",
                                                        similarity * 100.0
                                                    ),
                                                    occurred_at: self.clock.now(),
                                                },
                                            ),
                                        )
                                        .await?;
                                    progress
                                        .warn(format!(
                                            "discarded: low excerpt similarity {:.1}%",
                                            similarity * 100.0
                                        ))
                                        .await;
                                }
                                QuestionFilterDecision::RejectedDuplicate { similarity } => {
                                    rejection_attempt += 1;
                                    self.dataset_command_handler
                                        .handle(
                                            dataset_id,
                                            EvaluationDatasetCommand::RejectQuestion(
                                                RejectQuestion {
                                                    attempt: rejection_attempt,
                                                    reason: format!(
                                                        "duplicate similarity {:.1}%",
                                                        similarity * 100.0
                                                    ),
                                                    occurred_at: self.clock.now(),
                                                },
                                            ),
                                        )
                                        .await?;
                                    progress
                                        .warn(format!(
                                            "discarded: duplicate similarity {:.1}%",
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

        if gate.kept_count() == 0 {
            self.dataset_command_handler
                .handle(
                    dataset_id,
                    EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
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
            self.dataset_command_handler
                .handle(
                    dataset_id,
                    EvaluationDatasetCommand::FailDatasetGeneration(FailDatasetGeneration {
                        reason: reason.clone(),
                        occurred_at: self.clock.now(),
                    }),
                )
                .await?;
            return Err(AppError::Upstream(reason));
        }

        let stats = gate.stats();

        // Consume gate — questions were already issued as AcceptQuestion commands above.
        // Phase-1 limitation: question embeddings are not yet stored on the event
        // (the gate holds them in memory). Phase-2 will add a question_embeddings table.
        let _questions = gate.into_questions(target);

        self.dataset_command_handler
            .handle(
                dataset_id,
                EvaluationDatasetCommand::CompleteDatasetGeneration(CompleteDatasetGeneration {
                    occurred_at: self.clock.now(),
                }),
            )
            .await?;

        progress
            .success(format!(
                "dataset generated: {} question(s) accepted, {} rejected",
                target,
                stats.low_excerpt_similarity + stats.duplicate
            ))
            .await;

        Ok(dataset_id)
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
