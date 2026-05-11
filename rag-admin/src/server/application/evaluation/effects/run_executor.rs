use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};
use uuid::Uuid;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::configuration::PipelineResolver;
use crate::server::application::embedding::{EmbeddingService, ResolvedEmbeddingModel};
use crate::server::application::evaluation::retrieval::{
    cosine_similarity, retrieve_chunks, EvalChunk,
};
use crate::server::application::ports::{Clock, IdGenerator};
use crate::server::application::source_document::ports::{
    BlobStore, ChunkSetRepository, EmbeddingSetRepository,
};
use crate::server::application::AppError;
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};
use crate::server::domain::evaluation::dataset::repository::EvaluationDatasetRepository;
use crate::server::domain::evaluation::question::EvaluationQuestion;
use crate::server::domain::evaluation::run::aggregate::EvaluationRun;
use crate::server::domain::evaluation::run::commands::{
    CompleteRun, EvaluationRunCommand, FailRun, MarkVariantPrepared, ScoreVariant,
};
use crate::server::domain::evaluation::run::events::RetrievalTraceEntry;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::event_sourcing::command_processor::CommandProcessor;
use crate::server::event_sourcing::process_manager::EffectExecutor;
use crate::shared::{
    ChunkingVariant, EvaluationMetrics, EvaluationResultSplit, EvaluationRunOptions,
};

use super::run::{EvaluationRunEffect, ExecuteRunEffect};

pub struct EvaluationRunEffectExecutor {
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<dyn BlobStore>,
    chunker_registry: Arc<ChunkerRegistry>,
    chunk_set_repository: Arc<dyn ChunkSetRepository>,
    embedding_service: Arc<EmbeddingService>,
    embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    dataset_repository: Arc<dyn EvaluationDatasetRepository>,
    command_processor: Arc<CommandProcessor<EvaluationRun>>,
    pipeline_resolver: Arc<PipelineResolver>,
    clock: Arc<dyn Clock>,
    id_generator: Arc<dyn IdGenerator>,
}

impl EvaluationRunEffectExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_document_repository: Arc<dyn SourceDocumentRepository>,
        blob_store: Arc<dyn BlobStore>,
        chunker_registry: Arc<ChunkerRegistry>,
        chunk_set_repository: Arc<dyn ChunkSetRepository>,
        embedding_service: Arc<EmbeddingService>,
        embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
        dataset_repository: Arc<dyn EvaluationDatasetRepository>,
        command_processor: Arc<CommandProcessor<EvaluationRun>>,
        pipeline_resolver: Arc<PipelineResolver>,
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
            command_processor,
            pipeline_resolver,
            clock,
            id_generator,
        })
    }

    async fn run(&self, effect: &ExecuteRunEffect) -> Result<(), AppError> {
        if let Err(e) = self.run_inner(effect).await {
            error!(
                run_id = %effect.run_id,
                dataset_id = %effect.dataset_id,
                error = %e,
                "evaluation run failed"
            );
            // Best-effort: tell the run aggregate we failed so the read model
            // and process manager stop waiting on us. Swallow the secondary
            // error so we still surface the original `e` to the ledger.
            let _ = self
                .command_processor
                .handle(
                    effect.run_id,
                    EvaluationRunCommand::FailRun(FailRun {
                        run_id: effect.run_id,
                        reason: e.to_string(),
                        occurred_at: self.clock.now(),
                    }),
                )
                .await;
            return Err(e);
        }
        Ok(())
    }

    async fn run_inner(&self, effect: &ExecuteRunEffect) -> Result<(), AppError> {
        if effect.autotune_request.is_some() {
            return Err(AppError::Validation(
                "autotune is not yet implemented in the new evaluation path".into(),
            ));
        }

        let dataset = self
            .dataset_repository
            .load(effect.dataset_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("evaluation dataset {}", effect.dataset_id)))?;

        let questions = self
            .dataset_repository
            .load_questions(effect.dataset_id)
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

        let pipeline = self
            .pipeline_resolver
            .resolve(effect.pipeline_configuration_id)
            .await?;
        let embedding_model = &pipeline.embedding_model;

        let question_texts: Vec<String> = questions.iter().map(|q| q.question.clone()).collect();

        info!(
            run_id = %effect.run_id,
            dataset_id = %effect.dataset_id,
            questions = question_texts.len(),
            variants = effect.variants.len(),
            options = effect.options.len(),
            embedding_model = %embedding_model.model,
            "starting evaluation run"
        );

        let question_embeddings = self
            .embedding_service
            .embed_with_resolved(embedding_model, &question_texts)
            .await?;

        for variant in &effect.variants {
            info!(
                run_id = %effect.run_id,
                variant = %variant.label,
                "preparing variant"
            );

            let (chunk_set_id, chunks) = self
                .find_or_create_chunk_set(
                    dataset.document_id,
                    dataset.document_version,
                    &plain_text,
                    variant,
                )
                .await?;

            let (embedding_set_id, chunk_embeddings) = self
                .find_or_create_embedding_set(chunk_set_id, &chunks, embedding_model)
                .await?;

            self.command_processor
                .handle(
                    effect.run_id,
                    EvaluationRunCommand::MarkVariantPrepared(MarkVariantPrepared {
                        run_id: effect.run_id,
                        variant_label: variant.label.clone(),
                        chunk_set_id,
                        embedding_set_id,
                        occurred_at: self.clock.now(),
                    }),
                )
                .await?;

            info!(
                run_id = %effect.run_id,
                variant = %variant.label,
                chunks = chunks.len(),
                %chunk_set_id,
                %embedding_set_id,
                "variant prepared"
            );

            let splits = vec![EvaluationResultSplit::Full];

            for options in &effect.options {
                for split in &splits {
                    let (metrics, traces) = score_variant(
                        &questions,
                        &chunks,
                        &chunk_embeddings,
                        &question_embeddings,
                        options,
                    );

                    info!(
                        run_id = %effect.run_id,
                        variant = %variant.label,
                        split = split.as_str(),
                        top_k = options.top_k,
                        recall_mean = format!("{:.3}", metrics.recall_mean),
                        precision_mean = format!("{:.3}", metrics.precision_mean),
                        iou_mean = format!("{:.3}", metrics.iou_mean),
                        "variant scored"
                    );

                    self.command_processor
                        .handle(
                            effect.run_id,
                            EvaluationRunCommand::ScoreVariant(ScoreVariant {
                                run_id: effect.run_id,
                                variant_label: variant.label.clone(),
                                variant_config: variant.config.clone(),
                                options: options.clone(),
                                split: *split,
                                chunk_set_id,
                                embedding_set_id,
                                metrics,
                                retrieval_traces: traces,
                                selected: false,
                                occurred_at: self.clock.now(),
                            }),
                        )
                        .await?;
                }
            }
        }

        self.command_processor
            .handle(
                effect.run_id,
                EvaluationRunCommand::CompleteRun(CompleteRun {
                    run_id: effect.run_id,
                    occurred_at: self.clock.now(),
                }),
            )
            .await?;

        info!(
            run_id = %effect.run_id,
            variants = effect.variants.len(),
            "evaluation run complete"
        );

        Ok(())
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
        embedding_model: &ResolvedEmbeddingModel,
    ) -> Result<(Uuid, Vec<Vec<f32>>), AppError> {
        if let Some(existing) = self
            .embedding_set_repository
            .find_by(chunk_set_id, embedding_model.embedding_model_id)
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
            .embed_with_resolved(embedding_model, &texts)
            .await?;

        let embedding_set_id = self.id_generator.new_uuid();
        let occurred_at = self.clock.now();
        let embedding_set = EmbeddingSet {
            embedding_set_id,
            chunk_set_id,
            embedding_model_id: embedding_model.embedding_model_id,
            embedding_model_snapshot:
                crate::server::domain::configuration::embedding_model::EmbeddingModel {
                    embedding_model_id: embedding_model.embedding_model_id,
                    kind: embedding_model.kind,
                    model: embedding_model.model.clone(),
                    dimensions: embedding_model.dimensions,
                },
            dimensions: embedding_model.dimensions,
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

#[async_trait]
impl EffectExecutor<EvaluationRunEffect> for EvaluationRunEffectExecutor {
    async fn execute(&self, effect: &EvaluationRunEffect) -> Result<(), AppError> {
        match effect {
            EvaluationRunEffect::ExecuteRun(e) => self.run(e).await,
        }
    }
}

fn score_variant(
    questions: &[EvaluationQuestion],
    chunks: &[Chunk],
    chunk_embeddings: &[Vec<f32>],
    question_embeddings: &[Vec<f32>],
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
