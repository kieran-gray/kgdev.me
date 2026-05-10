use std::sync::Arc;

use uuid::Uuid;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::{
    command_handlers::EvaluationRunCommandHandler,
    progress::EvaluationProgress,
    retrieval::{cosine_similarity, retrieve_chunks, EvalChunk},
};
use crate::server::application::ports::{Clock, IdGenerator};
use crate::server::application::source_document::ports::{
    BlobStore, ChunkSetRepository, EmbeddingSetRepository,
};
use crate::server::application::AppError;
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};
use crate::server::domain::configuration::pipeline_configuration::PipelineConfigurationRepository;
use crate::server::domain::configuration::ConfigurationRepository;
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};
use crate::server::domain::evaluation::{
    dataset::repository::EvaluationDatasetRepository,
    question::EvaluationQuestion,
    run::{
        aggregate::EvaluationRun,
        commands::{
            CompleteRun, EvaluationRunCommand, MarkVariantPrepared, RequestRun, ScoreVariant,
        },
        events::RetrievalTraceEntry,
        read_model::EvaluationVariantResultDto,
        scoring_policy::ScoringPolicy,
    },
};
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::shared::{
    ChunkingVariant, EvaluationAutotuneRequest, EvaluationMetrics, EvaluationResultSplit,
    EvaluationRunOptions,
};

pub struct RunEvaluationRequest {
    pub dataset_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub variants: Vec<ChunkingVariant>,
    pub options: Vec<EvaluationRunOptions>,
    pub autotune_request: Option<EvaluationAutotuneRequest>,
    pub scoring_policy: ScoringPolicy,
}

pub struct RunEvaluationUseCase {
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<dyn BlobStore>,
    chunker_registry: Arc<ChunkerRegistry>,
    chunk_set_repository: Arc<dyn ChunkSetRepository>,
    embedding_service: Arc<EmbeddingService>,
    embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    dataset_repository: Arc<dyn EvaluationDatasetRepository>,
    run_command_handler: Arc<EvaluationRunCommandHandler>,
    configuration_repository: Arc<dyn ConfigurationRepository>,
    pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
    clock: Arc<dyn Clock>,
    id_generator: Arc<dyn IdGenerator>,
}

impl RunEvaluationUseCase {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_document_repository: Arc<dyn SourceDocumentRepository>,
        blob_store: Arc<dyn BlobStore>,
        chunker_registry: Arc<ChunkerRegistry>,
        chunk_set_repository: Arc<dyn ChunkSetRepository>,
        embedding_service: Arc<EmbeddingService>,
        embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
        dataset_repository: Arc<dyn EvaluationDatasetRepository>,
        run_command_handler: Arc<EvaluationRunCommandHandler>,
        configuration_repository: Arc<dyn ConfigurationRepository>,
        pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
        clock: Arc<dyn Clock>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Arc<Self> {
        Arc::new(Self {
            source_document_repository,
            blob_store,
            chunker_registry,
            chunk_set_repository,
            embedding_service,
            embedding_set_repository,
            dataset_repository,
            run_command_handler,
            configuration_repository,
            pipeline_configuration_repository,
            clock,
            id_generator,
        })
    }

    pub async fn execute(
        &self,
        request: RunEvaluationRequest,
        progress: Option<Arc<dyn EvaluationProgress>>,
    ) -> Result<Uuid, AppError> {
        if request.variants.is_empty() {
            return Err(AppError::Validation(
                "at least one chunking variant is required".into(),
            ));
        }
        if request.options.is_empty() {
            return Err(AppError::Validation(
                "at least one option set is required".into(),
            ));
        }

        let dataset = self
            .dataset_repository
            .load(request.dataset_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound(format!("evaluation dataset {}", request.dataset_id))
            })?;

        let questions = self
            .dataset_repository
            .load_questions(request.dataset_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if questions.is_empty() {
            return Err(AppError::Validation(
                "evaluation dataset has no questions".into(),
            ));
        }

        let doc = self
            .source_document_repository
            .load(dataset.document_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("document {}", dataset.document_id)))?;

        let bytes = self.blob_store.get(&doc.latest_content_hash).await?;
        let plain_text = String::from_utf8(bytes)
            .map_err(|e| AppError::Internal(format!("document content is not valid UTF-8: {e}")))?;

        let pipeline_configs = self
            .pipeline_configuration_repository
            .load_all()
            .await
            .map_err(|e| AppError::Internal(format!("failed to load pipeline configs: {e}")))?;

        let pc = pipeline_configs
            .iter()
            .find(|p| p.pipeline_configuration_id == request.pipeline_configuration_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "pipeline configuration {}",
                    request.pipeline_configuration_id
                ))
            })?;

        let config = self
            .configuration_repository
            .load()
            .await
            .map_err(|e| AppError::Internal(format!("failed to load configuration: {e}")))?;

        let embedding_model = config
            .embedding_models
            .iter()
            .find(|m| m.embedding_model_id == pc.embedding_model_id)
            .ok_or_else(|| AppError::NotFound("embedding model not found".into()))?
            .clone();

        let run_id = EvaluationRun::compute_id(
            request.dataset_id,
            request.pipeline_configuration_id,
            &request.variants,
            &request.options,
            request.autotune_request.as_ref(),
        );

        self.run_command_handler
            .handle(
                run_id,
                EvaluationRunCommand::RequestRun(RequestRun {
                    run_id,
                    dataset_id: request.dataset_id,
                    pipeline_configuration_id: request.pipeline_configuration_id,
                    document_id: dataset.document_id,
                    document_version: dataset.document_version,
                    variants: request.variants.clone(),
                    options: request.options.clone(),
                    autotune_request: request.autotune_request.clone(),
                    scoring_policy: request.scoring_policy,
                    occurred_at: self.clock.now(),
                }),
            )
            .await?;

        // Embed questions for retrieval.
        let question_texts: Vec<String> = questions.iter().map(|q| q.question.clone()).collect();
        let shared_model = crate::shared::EmbeddingModel {
            id: embedding_model.model.clone(),
            ..Default::default()
        };
        let question_embeddings = self
            .embedding_service
            .embed_batch(&shared_model, &question_texts)
            .await?;

        // For each variant: prepare (chunk + embed) then score.
        for variant in &request.variants {
            if let Some(ref p) = progress {
                p.info(format!("preparing variant '{}'...", variant.label))
                    .await;
            }

            let (chunk_set_id, chunks) = self
                .find_or_create_chunk_set(
                    dataset.document_id,
                    dataset.document_version,
                    &plain_text,
                    variant,
                )
                .await?;

            let (embedding_set_id, chunk_embeddings) = self
                .find_or_create_embedding_set(
                    chunk_set_id,
                    &chunks,
                    embedding_model.embedding_model_id,
                    &shared_model,
                )
                .await?;

            self.run_command_handler
                .handle(
                    run_id,
                    EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
                        variant_label: variant.label.clone(),
                        chunk_set_id,
                        embedding_set_id,
                        occurred_at: self.clock.now(),
                    }),
                )
                .await?;

            if let Some(ref p) = progress {
                p.info(format!("scoring variant '{}'...", variant.label))
                    .await;
            }

            // Score against each option set (for matrix runs).
            let splits = if request.autotune_request.is_some() {
                vec![
                    EvaluationResultSplit::Tuning,
                    EvaluationResultSplit::Holdout,
                ]
            } else {
                vec![EvaluationResultSplit::Full]
            };

            for options in &request.options {
                for split in &splits {
                    let (metrics, traces) = score_variant(
                        &questions,
                        &chunks,
                        &chunk_embeddings,
                        &question_embeddings,
                        chunk_set_id,
                        options,
                    );

                    let variant_result = EvaluationVariantResultDto {
                        run_id,
                        variant_label: variant.label.clone(),
                        split: *split,
                        recall_mean: metrics.recall_mean,
                        recall_std: metrics.recall_std,
                        precision_mean: metrics.precision_mean,
                        precision_std: metrics.precision_std,
                        iou_mean: metrics.iou_mean,
                        iou_std: metrics.iou_std,
                        precision_omega_mean: metrics.precision_omega_mean,
                        precision_omega_std: metrics.precision_omega_std,
                        chunk_set_id,
                        embedding_set_id,
                        selected: false,
                        retrieval_traces: traces,
                    };

                    self.run_command_handler
                        .handle_score_variant(
                            run_id,
                            EvaluationRunCommand::ScoreVariant(ScoreVariant {
                                variant_label: variant.label.clone(),
                                split: *split,
                                metrics,
                                retrieval_traces: variant_result.retrieval_traces.clone(),
                                selected: false,
                                occurred_at: self.clock.now(),
                            }),
                            variant_result,
                        )
                        .await?;
                }
            }
        }

        self.run_command_handler
            .handle(
                run_id,
                EvaluationRunCommand::CompleteRun(CompleteRun {
                    occurred_at: self.clock.now(),
                }),
            )
            .await?;

        if let Some(ref p) = progress {
            p.success("evaluation complete".to_string()).await;
        }

        Ok(run_id)
    }

    async fn find_or_create_chunk_set(
        &self,
        document_id: Uuid,
        document_version: u32,
        plain_text: &str,
        variant: &ChunkingVariant,
    ) -> Result<(Uuid, Vec<Chunk>), AppError> {
        let existing = self
            .chunk_set_repository
            .list_for_document(document_id)
            .await?;

        if let Some(cs) = existing.iter().find(|cs| {
            cs.document_version == document_version && cs.chunking_config == variant.config
        }) {
            let chunks = self
                .chunk_set_repository
                .load_chunks(cs.chunk_set_id)
                .await?;
            return Ok((cs.chunk_set_id, chunks));
        }

        let chunk_outputs = self
            .chunker_registry
            .chunk_markdown(&variant.config, plain_text)
            .await
            .map_err(|e| AppError::Internal(format!("chunking failed: {e}")))?;

        let chunk_set_id = self.id_generator.new_uuid();
        let occurred_at = self.clock.now();
        let chunks: Vec<Chunk> = chunk_outputs
            .into_iter()
            .enumerate()
            .map(|(i, co)| Chunk {
                chunk_id: self.id_generator.new_uuid(),
                chunk_set_id,
                sequence: i as u32,
                heading: co.heading,
                text: co.text,
                char_start: co.char_start,
                char_end: co.char_end,
            })
            .collect();

        let chunk_set = ChunkSet {
            chunk_set_id,
            document_id,
            document_version,
            chunking_config: variant.config.clone(),
            created_at: occurred_at.to_string(),
        };
        self.chunk_set_repository
            .save(chunk_set, chunks.clone())
            .await?;

        Ok((chunk_set_id, chunks))
    }

    async fn find_or_create_embedding_set(
        &self,
        chunk_set_id: Uuid,
        chunks: &[Chunk],
        embedding_model_id: Uuid,
        shared_model: &crate::shared::EmbeddingModel,
    ) -> Result<(Uuid, Vec<Vec<f32>>), AppError> {
        if let Some(existing) = self
            .embedding_set_repository
            .find_by(chunk_set_id, embedding_model_id)
            .await?
        {
            let embeddings = self
                .embedding_set_repository
                .load_embeddings(existing.embedding_set_id)
                .await?;
            let vecs: Vec<Vec<f32>> = embeddings.into_iter().map(|e| e.vector).collect();
            return Ok((existing.embedding_set_id, vecs));
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let vectors = self
            .embedding_service
            .embed_batch(shared_model, &texts)
            .await?;

        let embedding_set_id = self.id_generator.new_uuid();
        let occurred_at = self.clock.now();
        let embedding_set = EmbeddingSet {
            embedding_set_id,
            chunk_set_id,
            embedding_model_id,
            embedding_model_snapshot:
                crate::server::domain::configuration::embedding_model::EmbeddingModel {
                    embedding_model_id,
                    model: shared_model.id.clone(),
                    dimensions: shared_model.dims,
                    provider_id: Uuid::nil(),
                },
            dimensions: shared_model.dims,
            created_at: occurred_at.to_string(),
        };

        let embeddings: Vec<ChunkEmbedding> = chunks
            .iter()
            .zip(vectors.iter())
            .map(|(chunk, vec)| ChunkEmbedding {
                chunk_id: chunk.chunk_id,
                embedding_set_id,
                vector: vec.clone(),
            })
            .collect();

        self.embedding_set_repository
            .save(embedding_set, embeddings)
            .await?;

        Ok((embedding_set_id, vectors))
    }
}

fn score_variant(
    questions: &[EvaluationQuestion],
    chunks: &[Chunk],
    chunk_embeddings: &[Vec<f32>],
    question_embeddings: &[Vec<f32>],
    _chunk_set_id: Uuid,
    options: &EvaluationRunOptions,
) -> (EvaluationMetrics, Vec<RetrievalTraceEntry>) {
    let eval_chunks: Vec<EvalChunk> = chunks
        .iter()
        .enumerate()
        .map(|(i, c)| EvalChunk {
            chunk_id: i as u32,
            text: c.text.clone(),
            token_count: 0,
            char_start: c.char_start,
            char_end: c.char_end,
            body_chunk: true,
        })
        .collect();

    let mut recall_scores = Vec::new();
    let mut precision_scores = Vec::new();
    let mut iou_scores = Vec::new();
    let mut omega_scores = Vec::new();
    let mut traces = Vec::new();

    for (q_idx, (question, q_emb)) in questions.iter().zip(question_embeddings).enumerate() {
        let retrieved = retrieve_chunks(q_emb, &eval_chunks, chunk_embeddings, options);
        let retrieved_refs: Vec<&EvalChunk> = retrieved
            .iter()
            .map(|r| &eval_chunks[r.chunk_index])
            .collect();

        let retrieved_chunk_ids: Vec<Uuid> = retrieved
            .iter()
            .map(|r| chunks[r.chunk_index].chunk_id)
            .collect();
        let scores: Vec<f32> = retrieved
            .iter()
            .map(|r| cosine_similarity(q_emb, &chunk_embeddings[r.chunk_index]))
            .collect();

        let (recall, precision, iou) = score_question(question, &retrieved_refs);
        let omega = precision_omega(question, &eval_chunks);

        recall_scores.push(recall);
        precision_scores.push(precision);
        iou_scores.push(iou);
        omega_scores.push(omega);

        traces.push(RetrievalTraceEntry {
            question_sequence: q_idx as u32,
            retrieved_chunk_ids,
            scores,
            recall,
            precision,
            iou,
        });
    }

    let metrics = EvaluationMetrics {
        recall_mean: mean(&recall_scores),
        recall_std: std_dev(&recall_scores),
        precision_mean: mean(&precision_scores),
        precision_std: std_dev(&precision_scores),
        iou_mean: mean(&iou_scores),
        iou_std: std_dev(&iou_scores),
        precision_omega_mean: mean(&omega_scores),
        precision_omega_std: std_dev(&omega_scores),
    };

    (metrics, traces)
}

fn score_question(question: &EvaluationQuestion, retrieved: &[&EvalChunk]) -> (f32, f32, f32) {
    let reference_ranges: Vec<(u32, u32)> = question
        .references
        .iter()
        .filter(|r| r.char_end > r.char_start)
        .map(|r| (r.char_start, r.char_end))
        .collect();

    let relevant_len: u32 = non_overlapping_len(&reference_ranges);
    if relevant_len == 0 {
        return (0.0, 0.0, 0.0);
    }

    let mut intersection_len = 0u32;
    for chunk in retrieved.iter().filter(|c| c.body_chunk) {
        for &(ref_start, ref_end) in &reference_ranges {
            let overlap_start = chunk.char_start.max(ref_start);
            let overlap_end = chunk.char_end.min(ref_end);
            if overlap_end > overlap_start {
                intersection_len += overlap_end - overlap_start;
            }
        }
    }
    let intersection_len = intersection_len.min(relevant_len);

    let retrieved_len: u32 = retrieved.iter().map(|c| c.char_end - c.char_start).sum();
    let recall = intersection_len as f32 / relevant_len as f32;
    let precision = if retrieved_len == 0 {
        0.0
    } else {
        intersection_len as f32 / retrieved_len as f32
    };
    let iou_denom = retrieved_len + relevant_len - intersection_len;
    let iou = if iou_denom == 0 {
        0.0
    } else {
        intersection_len as f32 / iou_denom as f32
    };

    (recall, precision, iou)
}

fn precision_omega(question: &EvaluationQuestion, all_chunks: &[EvalChunk]) -> f32 {
    let reference_ranges: Vec<(u32, u32)> = question
        .references
        .iter()
        .filter(|r| r.char_end > r.char_start)
        .map(|r| (r.char_start, r.char_end))
        .collect();

    let relevant_len = non_overlapping_len(&reference_ranges);
    if relevant_len == 0 {
        return 0.0;
    }

    let min_possible: u32 = all_chunks
        .iter()
        .filter(|c| c.body_chunk)
        .map(|c| {
            let overlap: u32 = reference_ranges
                .iter()
                .map(|&(rs, re)| {
                    let os = c.char_start.max(rs);
                    let oe = c.char_end.min(re);
                    oe.saturating_sub(os)
                })
                .sum();
            if overlap > 0 {
                c.char_end - c.char_start
            } else {
                0
            }
        })
        .sum();

    if min_possible == 0 {
        0.0
    } else {
        relevant_len as f32 / min_possible as f32
    }
}

fn non_overlapping_len(ranges: &[(u32, u32)]) -> u32 {
    if ranges.is_empty() {
        return 0;
    }
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|&(s, _)| s);
    let mut total = 0u32;
    let mut cur_end = 0u32;
    for (s, e) in sorted {
        if s >= cur_end {
            total += e - s;
            cur_end = e;
        } else if e > cur_end {
            total += e - cur_end;
            cur_end = e;
        }
    }
    total
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

fn std_dev(values: &[f32]) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f32>() / values.len() as f32;
    variance.sqrt()
}
