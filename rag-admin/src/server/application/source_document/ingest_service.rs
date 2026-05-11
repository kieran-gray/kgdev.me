use std::collections::HashSet;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::configuration::{PipelineResolver, ResolvedPipeline};
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::ingest::VectorIndexResolver;
use crate::server::application::ports::{Clock, IdGenerator};
use crate::server::application::{AppError, IngestLogEvent, Job, JobRegistry};
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};
use crate::server::domain::indexing::aggregate::Indexing;
use crate::server::domain::indexing::commands::{
    CompleteChunking, CompleteEmbedding, CompleteIndexing, IndexingCommand, RequestIngest,
};
use crate::server::domain::indexing::repository::IndexingRepository;
use crate::server::domain::source_document::commands::{
    AddVersion, CreateDocument, NewVersion, SourceDocumentCommand,
};
use crate::server::domain::source_document::document_type::DocumentType;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::domain::source_document::source_ref::SourceRef;
use crate::server::domain::VectorRecord;
use crate::shared::{ChunkingConfig, IngestJobInfo, SourceDocumentDto};

use super::{
    command_handler::SourceDocumentCommandHandler,
    ports::{BlobStore, ChunkSetRepository, EmbeddingSetRepository, SourceAdapterRegistry},
};
use crate::server::application::indexing::command_handler::IndexingCommandHandler;

const EMBED_BATCH: usize = 50;
const UPSERT_BATCH: usize = 100;

pub struct SourceDocumentIngestServiceDeps {
    pub source_document_command_handler: Arc<SourceDocumentCommandHandler>,
    pub indexing_command_handler: Arc<IndexingCommandHandler>,
    pub source_document_repository: Arc<dyn SourceDocumentRepository>,
    pub indexing_repository: Arc<dyn IndexingRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub chunk_set_repository: Arc<dyn ChunkSetRepository>,
    pub embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
    pub chunker_registry: Arc<ChunkerRegistry>,
    pub embedding_service: Arc<EmbeddingService>,
    pub vector_index_resolver: Arc<VectorIndexResolver>,
    pub pipeline_resolver: Arc<PipelineResolver>,
    pub job_registry: Arc<JobRegistry>,
    pub clock: Arc<dyn Clock>,
    pub id_generator: Arc<dyn IdGenerator>,
}

pub struct SourceDocumentIngestService {
    source_document_command_handler: Arc<SourceDocumentCommandHandler>,
    indexing_command_handler: Arc<IndexingCommandHandler>,
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    indexing_repository: Arc<dyn IndexingRepository>,
    blob_store: Arc<dyn BlobStore>,
    chunk_set_repository: Arc<dyn ChunkSetRepository>,
    embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    source_adapter_registry: Arc<SourceAdapterRegistry>,
    chunker_registry: Arc<ChunkerRegistry>,
    embedding_service: Arc<EmbeddingService>,
    vector_index_resolver: Arc<VectorIndexResolver>,
    pipeline_resolver: Arc<PipelineResolver>,
    job_registry: Arc<JobRegistry>,
    clock: Arc<dyn Clock>,
    id_generator: Arc<dyn IdGenerator>,
    running: Mutex<HashSet<String>>,
}

impl SourceDocumentIngestService {
    pub fn new(deps: SourceDocumentIngestServiceDeps) -> Arc<Self> {
        Arc::new(Self {
            source_document_command_handler: deps.source_document_command_handler,
            indexing_command_handler: deps.indexing_command_handler,
            source_document_repository: deps.source_document_repository,
            indexing_repository: deps.indexing_repository,
            blob_store: deps.blob_store,
            chunk_set_repository: deps.chunk_set_repository,
            embedding_set_repository: deps.embedding_set_repository,
            source_adapter_registry: deps.source_adapter_registry,
            chunker_registry: deps.chunker_registry,
            embedding_service: deps.embedding_service,
            vector_index_resolver: deps.vector_index_resolver,
            pipeline_resolver: deps.pipeline_resolver,
            job_registry: deps.job_registry,
            clock: deps.clock,
            id_generator: deps.id_generator,
            running: Mutex::new(HashSet::new()),
        })
    }

    // ── Public: import (register / version only, no indexing) ─────────────

    /// Import a document from its source adapter: fetch upstream, store the
    /// content blob, and create (or version) the SourceDocument aggregate.
    ///
    /// This is the prerequisite for any later experimentation. Returns the
    /// resulting document DTO. Idempotent: identical content_hash → reuses the
    /// existing version.
    pub async fn import_document(
        &self,
        source_ref: SourceRef,
        document_type: DocumentType,
    ) -> Result<SourceDocumentDto, AppError> {
        let occurred_at = self.clock.now();
        let adapter = self
            .source_adapter_registry
            .get(&document_type)
            .ok_or_else(|| {
                AppError::Validation(format!("no adapter registered for {document_type:?}"))
            })?;
        let fetched = adapter
            .fetch(&source_ref)
            .await
            .map_err(|e| AppError::Upstream(format!("fetch failed: {e}")))?;
        let content_hash = self.blob_store.put(&fetched.content).await?;

        let existing = self
            .source_document_repository
            .find_by_source_ref(&source_ref)
            .await?;

        let (document_id, document_version) = match existing {
            None => {
                let document_id = self.id_generator.new_uuid();
                self.source_document_command_handler
                    .handle(SourceDocumentCommand::CreateDocument(CreateDocument {
                        document_id,
                        document_type: document_type.clone(),
                        source_ref: source_ref.clone(),
                        initial_version: NewVersion {
                            content_hash: content_hash.clone(),
                            metadata: fetched.metadata.clone(),
                        },
                        occurred_at: occurred_at.clone(),
                    }))
                    .await?;
                (document_id, 1u32)
            }
            Some(existing_doc) => {
                if existing_doc.latest_content_hash == content_hash {
                    (existing_doc.document_id, existing_doc.latest_version_number)
                } else {
                    self.source_document_command_handler
                        .handle(SourceDocumentCommand::AddVersion(AddVersion {
                            document_id: existing_doc.document_id,
                            version: NewVersion {
                                content_hash: content_hash.clone(),
                                metadata: fetched.metadata.clone(),
                            },
                            occurred_at: occurred_at.clone(),
                        }))
                        .await?;
                    (
                        existing_doc.document_id,
                        existing_doc.latest_version_number + 1,
                    )
                }
            }
        };

        let title = match &fetched.metadata {
            crate::server::domain::source_document::version::DocumentMetadata::BlogPost(meta) => {
                meta.title.clone()
            }
        };

        Ok(SourceDocumentDto {
            document_id,
            document_type: format!("{document_type:?}"),
            source_ref_key: source_ref.natural_key().to_string(),
            title,
            latest_version: document_version,
            latest_content_hash: content_hash.as_hex().to_string(),
            deleted: false,
        })
    }

    // ── Public: per-stage launchers ───────────────────────────────────────

    /// Request a new indexing (or restart an existing one): creates the
    /// Indexing aggregate in Pending state. Does NOT run any stages.
    /// Caller drives stages explicitly via `start_chunking_stage` etc.
    pub async fn request_indexing(
        &self,
        source_ref: SourceRef,
        document_type: DocumentType,
        pipeline_configuration_id: Uuid,
        chunking_config: ChunkingConfig,
    ) -> Result<Uuid, AppError> {
        let document = self
            .source_document_repository
            .find_by_source_ref(&source_ref)
            .await?
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "document {} is not imported yet; call import_document first",
                    source_ref.natural_key()
                ))
            })?;

        // Validate pipeline exists upfront so the operator gets a clear error.
        let _ = self.pipeline_resolver.resolve(pipeline_configuration_id).await?;

        // Document-type sanity: the adapter call below would fail anyway, but
        // catch it early.
        let _ = document_type;

        let occurred_at = self.clock.now();
        let request_id = self.id_generator.new_uuid();
        let indexing_id =
            Indexing::compute_id(document.document_id, pipeline_configuration_id);

        self.indexing_command_handler
            .handle(IndexingCommand::RequestIngest(RequestIngest {
                document_id: document.document_id,
                pipeline_configuration_id,
                document_version: document.latest_version_number,
                chunking_config,
                request_id,
                occurred_at,
            }))
            .await?;

        Ok(indexing_id)
    }

    pub async fn start_chunking_stage(
        self: &Arc<Self>,
        indexing_id: Uuid,
    ) -> Result<IngestJobInfo, AppError> {
        let key = format!("chunking:{indexing_id}");
        self.spawn_stage_job(key, indexing_id, Self::run_chunking_stage_inner)
            .await
    }

    pub async fn start_embedding_stage(
        self: &Arc<Self>,
        indexing_id: Uuid,
    ) -> Result<IngestJobInfo, AppError> {
        let key = format!("embedding:{indexing_id}");
        self.spawn_stage_job(key, indexing_id, Self::run_embedding_stage_inner)
            .await
    }

    pub async fn start_upsert_stage(
        self: &Arc<Self>,
        indexing_id: Uuid,
    ) -> Result<IngestJobInfo, AppError> {
        let key = format!("upsert:{indexing_id}");
        self.spawn_stage_job(key, indexing_id, Self::run_upsert_stage_inner)
            .await
    }

    // ── Public: one-shot full ingest (back-compat) ────────────────────────

    /// One-shot: import → request_indexing → chunk → embed → upsert.
    /// Use this when the operator wants the old "do everything" button.
    pub async fn start_ingest(
        self: &Arc<Self>,
        source_ref: SourceRef,
        document_type: DocumentType,
        pipeline_configuration_id: Uuid,
        chunking_config: ChunkingConfig,
    ) -> Result<IngestJobInfo, AppError> {
        let run_key = format!("full:{}:{}", source_ref.natural_key(), pipeline_configuration_id);
        {
            let mut guard = self.running.lock().await;
            if guard.contains(&run_key) {
                return Err(AppError::Validation(format!(
                    "ingest for {} pipeline {} is already running",
                    source_ref.natural_key(),
                    pipeline_configuration_id
                )));
            }
            guard.insert(run_key.clone());
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            let result = svc
                .run_full_ingest_inner(
                    &source_ref,
                    document_type,
                    pipeline_configuration_id,
                    chunking_config,
                    job.clone(),
                )
                .await;
            if let Err(e) = &result {
                job.emit(IngestLogEvent::error(format!("ingest failed: {e}")))
                    .await;
            }
            job.finish().await;
            svc.running.lock().await.remove(&run_key);
        });

        Ok(IngestJobInfo { job_id, stream_url })
    }

    // ── Internal: stage job spawner ───────────────────────────────────────

    async fn spawn_stage_job<F, Fut>(
        self: &Arc<Self>,
        run_key: String,
        indexing_id: Uuid,
        runner: F,
    ) -> Result<IngestJobInfo, AppError>
    where
        F: FnOnce(Arc<Self>, Uuid, Arc<Job>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), AppError>> + Send + 'static,
    {
        {
            let mut guard = self.running.lock().await;
            if guard.contains(&run_key) {
                return Err(AppError::Validation(format!(
                    "{run_key} is already running"
                )));
            }
            guard.insert(run_key.clone());
        }

        let (job_id, job) = self.job_registry.create().await;
        let stream_url = format!("/api/ingest/logs/{job_id}");

        let svc = self.clone();
        tokio::spawn(async move {
            let result = runner(svc.clone(), indexing_id, job.clone()).await;
            if let Err(e) = &result {
                job.emit(IngestLogEvent::error(format!("stage failed: {e}")))
                    .await;
            }
            job.finish().await;
            svc.running.lock().await.remove(&run_key);
        });

        Ok(IngestJobInfo { job_id, stream_url })
    }

    // ── Internal: stage runners ───────────────────────────────────────────

    async fn run_chunking_stage_inner(
        self: Arc<Self>,
        indexing_id: Uuid,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let indexing = self.load_indexing(indexing_id).await?;
        if indexing.chunk_set_id.is_some() {
            job.emit(IngestLogEvent::info(format!(
                "chunking already complete (chunk_set={}); nothing to do",
                indexing.chunk_set_id.unwrap()
            )))
            .await;
            return Ok(());
        }

        let document = self
            .source_document_repository
            .load(indexing.document_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("document {} not found", indexing.document_id))
            })?;

        // Chunk from the stored markdown content. No upstream re-fetch — the
        // version we chunk for is the one we recorded at import.
        let bytes = self.blob_store.get(&document.latest_content_hash).await?;
        let markdown = String::from_utf8(bytes).map_err(|e| {
            AppError::Internal(format!("content for {} is not utf-8: {e}", document.document_id))
        })?;
        self.chunk_and_record(&indexing, &markdown, &job).await?;
        Ok(())
    }

    async fn run_embedding_stage_inner(
        self: Arc<Self>,
        indexing_id: Uuid,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let indexing = self.load_indexing(indexing_id).await?;
        let chunk_set_id = indexing.chunk_set_id.ok_or_else(|| {
            AppError::Validation("chunking has not completed; run the chunking stage first".into())
        })?;
        if indexing.embedding_set_id.is_some() {
            job.emit(IngestLogEvent::info(format!(
                "embedding already complete (embedding_set={}); nothing to do",
                indexing.embedding_set_id.unwrap()
            )))
            .await;
            return Ok(());
        }

        let pipeline = self
            .pipeline_resolver
            .resolve(indexing.pipeline_configuration_id)
            .await?;
        self.embed_and_record(&indexing, chunk_set_id, &pipeline, &job)
            .await?;
        Ok(())
    }

    async fn run_upsert_stage_inner(
        self: Arc<Self>,
        indexing_id: Uuid,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let indexing = self.load_indexing(indexing_id).await?;
        let embedding_set_id = indexing.embedding_set_id.ok_or_else(|| {
            AppError::Validation(
                "embedding has not completed; run the embedding stage first".into(),
            )
        })?;
        let chunk_set_id = indexing.chunk_set_id.ok_or_else(|| {
            AppError::Validation("chunk set missing; restart from chunking".into())
        })?;
        if indexing.status.is_indexed() {
            job.emit(IngestLogEvent::info("already indexed; nothing to do"))
                .await;
            return Ok(());
        }

        let pipeline = self
            .pipeline_resolver
            .resolve(indexing.pipeline_configuration_id)
            .await?;
        let chunks = self.chunk_set_repository.load_chunks(chunk_set_id).await?;
        self.upsert_and_record(&indexing, chunk_set_id, embedding_set_id, &chunks, &pipeline, &job)
            .await?;
        Ok(())
    }

    // ── Internal: full-ingest orchestrator ────────────────────────────────

    async fn run_full_ingest_inner(
        self: &Arc<Self>,
        source_ref: &SourceRef,
        document_type: DocumentType,
        pipeline_configuration_id: Uuid,
        chunking_config: ChunkingConfig,
        job: Arc<Job>,
    ) -> Result<(), AppError> {
        let occurred_at = self.clock.now();

        job.emit(IngestLogEvent::info(format!(
            "fetching '{}' from source…",
            source_ref.natural_key()
        )))
        .await;
        let adapter = self
            .source_adapter_registry
            .get(&document_type)
            .ok_or_else(|| {
                AppError::Validation(format!("no adapter registered for {document_type:?}"))
            })?;
        let fetched = adapter
            .fetch(source_ref)
            .await
            .map_err(|e| AppError::Upstream(format!("fetch failed: {e}")))?;
        let content_hash = self.blob_store.put(&fetched.content).await?;

        let existing = self
            .source_document_repository
            .find_by_source_ref(source_ref)
            .await?;

        let (document_id, document_version) = match existing {
            None => {
                job.emit(IngestLogEvent::info("creating new source document…"))
                    .await;
                let document_id = self.id_generator.new_uuid();
                self.source_document_command_handler
                    .handle(SourceDocumentCommand::CreateDocument(CreateDocument {
                        document_id,
                        document_type: document_type.clone(),
                        source_ref: source_ref.clone(),
                        initial_version: NewVersion {
                            content_hash: content_hash.clone(),
                            metadata: fetched.metadata.clone(),
                        },
                        occurred_at: occurred_at.clone(),
                    }))
                    .await?;
                (document_id, 1u32)
            }
            Some(existing_doc) => {
                let new_version = if existing_doc.latest_content_hash != content_hash {
                    job.emit(IngestLogEvent::info(format!(
                        "content changed ({}→{}), adding new version…",
                        &existing_doc.latest_content_hash.as_hex()[..8],
                        &content_hash.as_hex()[..8],
                    )))
                    .await;
                    self.source_document_command_handler
                        .handle(SourceDocumentCommand::AddVersion(AddVersion {
                            document_id: existing_doc.document_id,
                            version: NewVersion {
                                content_hash: content_hash.clone(),
                                metadata: fetched.metadata.clone(),
                            },
                            occurred_at: occurred_at.clone(),
                        }))
                        .await?;
                    existing_doc.latest_version_number + 1
                } else {
                    job.emit(IngestLogEvent::info(format!(
                        "content unchanged ({}), using existing version {}",
                        &content_hash.as_hex()[..8],
                        existing_doc.latest_version_number
                    )))
                    .await;
                    existing_doc.latest_version_number
                };
                (existing_doc.document_id, new_version)
            }
        };

        let pipeline = self.pipeline_resolver.resolve(pipeline_configuration_id).await?;
        job.emit(IngestLogEvent::info(format!(
            "pipeline: embedding={} ({} dims) → index={}",
            pipeline.embedding_model.model,
            pipeline.embedding_model.dimensions,
            pipeline.vector_index.name
        )))
        .await;

        let request_id = self.id_generator.new_uuid();
        let indexing_id = Indexing::compute_id(document_id, pipeline_configuration_id);

        self.indexing_command_handler
            .handle(IndexingCommand::RequestIngest(RequestIngest {
                document_id,
                pipeline_configuration_id,
                document_version,
                chunking_config: chunking_config.clone(),
                request_id,
                occurred_at: occurred_at.clone(),
            }))
            .await?;

        // Build a synthetic Indexing read model so the helpers can run without
        // a repository round-trip. The pieces match what the projector would
        // emit on RequestIngest.
        let indexing_rm = crate::server::domain::indexing::read_model::IndexingReadModel {
            indexing_id,
            document_id,
            pipeline_configuration_id,
            document_version,
            chunking_config: chunking_config.clone(),
            chunk_set_id: None,
            embedding_set_id: None,
            status: crate::server::domain::indexing::status::IndexingStatus::Pending,
            attempts: 1,
            removed: false,
        };

        let markdown = String::from_utf8(fetched.content.clone()).map_err(|e| {
            AppError::Internal(format!("fetched content is not utf-8: {e}"))
        })?;
        let chunk_set_id = self
            .chunk_and_record(&indexing_rm, &markdown, &job)
            .await?;
        let embedding_set_id = self
            .embed_and_record(&indexing_rm, chunk_set_id, &pipeline, &job)
            .await?;
        let chunks = self
            .chunk_set_repository
            .load_chunks(chunk_set_id)
            .await?;
        self.upsert_and_record(
            &indexing_rm,
            chunk_set_id,
            embedding_set_id,
            &chunks,
            &pipeline,
            &job,
        )
        .await?;

        Ok(())
    }

    // ── Internal: stage helpers (the actual work) ─────────────────────────

    async fn chunk_and_record(
        &self,
        indexing: &crate::server::domain::indexing::read_model::IndexingReadModel,
        markdown: &str,
        job: &Arc<Job>,
    ) -> Result<Uuid, AppError> {
        let occurred_at = self.clock.now();

        job.emit(IngestLogEvent::info(format!(
            "chunking with {}…",
            indexing.chunking_config.describe()
        )))
        .await;

        let chunk_outputs = self
            .chunker_registry
            .chunk_markdown(&indexing.chunking_config, markdown)
            .await
            .map_err(|e| AppError::Internal(format!("chunking failed: {e}")))?;

        let chunk_count = chunk_outputs.len() as u32;
        let chunk_set_id = self.id_generator.new_uuid();
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
            document_id: indexing.document_id,
            document_version: indexing.document_version,
            chunking_config: indexing.chunking_config,
            created_at: occurred_at.to_string(),
        };

        self.chunk_set_repository.save(chunk_set, chunks).await?;

        self.indexing_command_handler
            .handle_for(
                indexing.indexing_id,
                IndexingCommand::CompleteChunking(CompleteChunking {
                    chunk_set_id,
                    chunk_count,
                    occurred_at,
                }),
            )
            .await?;

        job.emit(IngestLogEvent::info(format!(
            "{chunk_count} chunks created"
        )))
        .await;

        Ok(chunk_set_id)
    }

    async fn embed_and_record(
        &self,
        indexing: &crate::server::domain::indexing::read_model::IndexingReadModel,
        chunk_set_id: Uuid,
        pipeline: &ResolvedPipeline,
        job: &Arc<Job>,
    ) -> Result<Uuid, AppError> {
        let occurred_at = self.clock.now();
        let chunks = self.chunk_set_repository.load_chunks(chunk_set_id).await?;
        let embedding_model = &pipeline.embedding_model;

        let embedding_set_id = if let Some(existing_set) = self
            .embedding_set_repository
            .find_by(chunk_set_id, embedding_model.embedding_model_id)
            .await?
        {
            job.emit(IngestLogEvent::info(format!(
                "reusing existing embedding set {} (skipping re-embed)",
                existing_set.embedding_set_id
            )))
            .await;
            existing_set.embedding_set_id
        } else {
            job.emit(IngestLogEvent::info(format!(
                "embedding {} chunks via {} ({} dims)…",
                chunks.len(),
                embedding_model.model,
                embedding_model.dimensions
            )))
            .await;

            let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
            let mut all_vectors: Vec<Vec<f32>> = Vec::with_capacity(texts.len());

            for (i, batch) in texts.chunks(EMBED_BATCH).enumerate() {
                job.emit(IngestLogEvent::info(format!(
                    "embedding batch {}/{} ({} chunks)…",
                    i + 1,
                    texts.len().div_ceil(EMBED_BATCH),
                    batch.len(),
                )))
                .await;
                let vecs = self
                    .embedding_service
                    .embed_with_resolved(embedding_model, batch)
                    .await?;
                all_vectors.extend(vecs);
            }

            let new_set_id = self.id_generator.new_uuid();
            let embedding_set = EmbeddingSet {
                embedding_set_id: new_set_id,
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
                .zip(all_vectors)
                .map(|(chunk, vector)| ChunkEmbedding {
                    chunk_id: chunk.chunk_id,
                    embedding_set_id: new_set_id,
                    vector,
                })
                .collect();

            self.embedding_set_repository
                .save(embedding_set, embeddings)
                .await?;

            new_set_id
        };

        self.indexing_command_handler
            .handle_for(
                indexing.indexing_id,
                IndexingCommand::CompleteEmbedding(CompleteEmbedding {
                    embedding_set_id,
                    occurred_at,
                }),
            )
            .await?;

        Ok(embedding_set_id)
    }

    async fn upsert_and_record(
        &self,
        indexing: &crate::server::domain::indexing::read_model::IndexingReadModel,
        chunk_set_id: Uuid,
        embedding_set_id: Uuid,
        chunks: &[Chunk],
        pipeline: &ResolvedPipeline,
        job: &Arc<Job>,
    ) -> Result<(), AppError> {
        let occurred_at = self.clock.now();
        let vector_index_name = pipeline.vector_index.name.as_str();
        job.emit(IngestLogEvent::info(format!(
            "upserting to index '{vector_index_name}'…"
        )))
        .await;

        let embeddings = self
            .embedding_set_repository
            .load_embeddings(embedding_set_id)
            .await?;

        let chunk_map: std::collections::HashMap<Uuid, &Chunk> =
            chunks.iter().map(|c| (c.chunk_id, c)).collect();

        let document_id = indexing.document_id;
        let pipeline_configuration_id = indexing.pipeline_configuration_id;
        let document_version = indexing.document_version;

        let records: Vec<VectorRecord> = embeddings
            .iter()
            .filter_map(|e| chunk_map.get(&e.chunk_id).map(|c| (e, *c)))
            .map(|(e, chunk)| VectorRecord {
                id: format!(
                    "{document_id}:{pipeline_configuration_id}:{}",
                    chunk.chunk_id
                ),
                values: e.vector.clone(),
                metadata: json!({
                    "document_id": document_id.to_string(),
                    "document_version": document_version,
                    "pipeline_configuration_id": pipeline_configuration_id.to_string(),
                    "chunk_id": chunk.chunk_id.to_string(),
                    "chunk_set_id": chunk_set_id.to_string(),
                    "heading": chunk.heading,
                    "text": chunk.text,
                    "char_start": chunk.char_start,
                    "char_end": chunk.char_end,
                }),
            })
            .collect();

        let vector_index = self.vector_index_resolver.build(&pipeline.vector_index)?;
        let vector_count = records.len() as u32;

        for (i, batch) in records.chunks(UPSERT_BATCH).enumerate() {
            job.emit(IngestLogEvent::info(format!(
                "upserting batch {}/{} ({} records)…",
                i + 1,
                records.len().div_ceil(UPSERT_BATCH),
                batch.len(),
            )))
            .await;
            vector_index.upsert(batch).await?;
        }

        self.indexing_command_handler
            .handle_for(
                indexing.indexing_id,
                IndexingCommand::CompleteIndexing(CompleteIndexing {
                    vector_count,
                    occurred_at,
                }),
            )
            .await?;

        job.emit(IngestLogEvent::success(format!(
            "upsert complete · {vector_count} vectors → '{vector_index_name}'"
        )))
        .await;

        Ok(())
    }

    // ── Internal: support ─────────────────────────────────────────────────

    async fn load_indexing(
        &self,
        indexing_id: Uuid,
    ) -> Result<crate::server::domain::indexing::read_model::IndexingReadModel, AppError> {
        self.indexing_repository
            .load(indexing_id)
            .await
            .map_err(|e| AppError::Internal(format!("load indexing: {e}")))?
            .ok_or_else(|| AppError::NotFound(format!("indexing {indexing_id} not found")))
    }

}
