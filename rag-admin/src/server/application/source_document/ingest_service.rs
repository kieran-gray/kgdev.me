use std::collections::HashSet;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::ports::{Clock, IdGenerator};
use crate::server::application::{AppError, IngestLogEvent, Job, JobRegistry};
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};
use crate::server::domain::configuration::embedding_model::EmbeddingModel;
use crate::server::domain::configuration::pipeline_configuration::{
    PipelineConfigurationRepository, PipelineConfigurationRepositoryError,
};
use crate::server::domain::configuration::{ConfigurationRepository, ConfigurationRepositoryError};
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};
use crate::server::domain::indexing::aggregate::Indexing;
use crate::server::domain::indexing::commands::{
    CompleteChunking, CompleteEmbedding, CompleteIndexing, IndexingCommand, RequestIngest,
};
use crate::server::domain::source_document::commands::{
    AddVersion, CreateDocument, NewVersion, SourceDocumentCommand,
};
use crate::server::domain::source_document::document_type::DocumentType;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::domain::source_document::source_ref::SourceRef;
use crate::server::domain::VectorRecord;
use crate::shared::{ChunkingConfig, IngestJobInfo};

use super::{
    command_handler::SourceDocumentCommandHandler,
    ports::{
        BlobStore, ChunkSetRepository, EmbeddingSetRepository, SourceAdapterRegistry,
        VectorIndexFactory,
    },
};
use crate::server::application::indexing::command_handler::IndexingCommandHandler;

const EMBED_BATCH: usize = 50;
const UPSERT_BATCH: usize = 100;

pub struct SourceDocumentIngestServiceDeps {
    pub source_document_command_handler: Arc<SourceDocumentCommandHandler>,
    pub indexing_command_handler: Arc<IndexingCommandHandler>,
    pub source_document_repository: Arc<dyn SourceDocumentRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub chunk_set_repository: Arc<dyn ChunkSetRepository>,
    pub embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
    pub chunker_registry: Arc<ChunkerRegistry>,
    pub embedding_service: Arc<EmbeddingService>,
    pub vector_index_factory: Arc<dyn VectorIndexFactory>,
    pub configuration_repository: Arc<dyn ConfigurationRepository>,
    pub pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
    pub job_registry: Arc<JobRegistry>,
    pub clock: Arc<dyn Clock>,
    pub id_generator: Arc<dyn IdGenerator>,
}

pub struct SourceDocumentIngestService {
    source_document_command_handler: Arc<SourceDocumentCommandHandler>,
    indexing_command_handler: Arc<IndexingCommandHandler>,
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<dyn BlobStore>,
    chunk_set_repository: Arc<dyn ChunkSetRepository>,
    embedding_set_repository: Arc<dyn EmbeddingSetRepository>,
    source_adapter_registry: Arc<SourceAdapterRegistry>,
    chunker_registry: Arc<ChunkerRegistry>,
    embedding_service: Arc<EmbeddingService>,
    vector_index_factory: Arc<dyn VectorIndexFactory>,
    configuration_repository: Arc<dyn ConfigurationRepository>,
    pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
    job_registry: Arc<JobRegistry>,
    clock: Arc<dyn Clock>,
    id_generator: Arc<dyn IdGenerator>,
    running: Mutex<HashSet<String>>,
}

// TODO: critical must store ingest status in KV so it can be checked in backend

impl SourceDocumentIngestService {
    pub fn new(deps: SourceDocumentIngestServiceDeps) -> Arc<Self> {
        Arc::new(Self {
            source_document_command_handler: deps.source_document_command_handler,
            indexing_command_handler: deps.indexing_command_handler,
            source_document_repository: deps.source_document_repository,
            blob_store: deps.blob_store,
            chunk_set_repository: deps.chunk_set_repository,
            embedding_set_repository: deps.embedding_set_repository,
            source_adapter_registry: deps.source_adapter_registry,
            chunker_registry: deps.chunker_registry,
            embedding_service: deps.embedding_service,
            vector_index_factory: deps.vector_index_factory,
            configuration_repository: deps.configuration_repository,
            pipeline_configuration_repository: deps.pipeline_configuration_repository,
            job_registry: deps.job_registry,
            clock: deps.clock,
            id_generator: deps.id_generator,
            running: Mutex::new(HashSet::new()),
        })
    }

    pub async fn start_ingest(
        self: &Arc<Self>,
        source_ref: SourceRef,
        document_type: DocumentType,
        pipeline_configuration_id: Uuid,
        chunking_config: ChunkingConfig,
    ) -> Result<IngestJobInfo, AppError> {
        let run_key = format!("{}:{}", source_ref.natural_key(), pipeline_configuration_id);
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
                .run_ingest(
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

    async fn run_ingest(
        &self,
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

        let fetched = adapter.fetch(source_ref).await.map_err(|e| {
            let err = AppError::Upstream(format!("fetch failed: {e}"));
            err
        })?;

        let existing = self
            .source_document_repository
            .find_by_source_ref(source_ref)
            .await?;

        let content_hash = self.blob_store.put(&fetched.content).await?;

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

        let (embedding_model, vector_index_name) =
            self.resolve_pipeline(pipeline_configuration_id).await?;

        job.emit(IngestLogEvent::info(format!(
            "pipeline: embedding={} ({} dims) → index={}",
            embedding_model.model, embedding_model.dimensions, vector_index_name
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

        job.emit(IngestLogEvent::info(format!(
            "chunking with {}…",
            chunking_config.describe()
        )))
        .await;

        let chunk_outputs = self
            .chunker_registry
            .chunk_markdown(&chunking_config, &fetched.plain_text)
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
            document_id,
            document_version,
            chunking_config: chunking_config.clone(),
            created_at: occurred_at.to_string(),
        };

        self.chunk_set_repository
            .save(chunk_set, chunks.clone())
            .await?;

        self.indexing_command_handler
            .handle_for(
                indexing_id,
                IndexingCommand::CompleteChunking(CompleteChunking {
                    chunk_set_id,
                    chunk_count,
                    occurred_at: occurred_at.clone(),
                }),
            )
            .await?;

        job.emit(IngestLogEvent::info(format!(
            "{chunk_count} chunks created"
        )))
        .await;

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
                chunk_count, embedding_model.model, embedding_model.dimensions
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
                let shared_model = crate::shared::EmbeddingModel {
                    id: embedding_model.model.clone(),
                    ..Default::default()
                };
                let vecs = self
                    .embedding_service
                    .embed_batch(&shared_model, batch)
                    .await?;
                all_vectors.extend(vecs);
            }

            let new_set_id = self.id_generator.new_uuid();
            let embedding_set = EmbeddingSet {
                embedding_set_id: new_set_id,
                chunk_set_id,
                embedding_model_id: embedding_model.embedding_model_id,
                embedding_model_snapshot: embedding_model.clone(),
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
                indexing_id,
                IndexingCommand::CompleteEmbedding(CompleteEmbedding {
                    embedding_set_id,
                    occurred_at: occurred_at.clone(),
                }),
            )
            .await?;

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

        let vector_index = self.vector_index_factory.for_index(&vector_index_name);
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
                indexing_id,
                IndexingCommand::CompleteIndexing(CompleteIndexing {
                    vector_count,
                    occurred_at: occurred_at.clone(),
                }),
            )
            .await?;

        job.emit(IngestLogEvent::success(format!(
            "ingest complete · {chunk_count} chunks · {vector_count} vectors · document_id={document_id}"
        )))
        .await;

        Ok(())
    }

    async fn resolve_pipeline(
        &self,
        pipeline_configuration_id: Uuid,
    ) -> Result<(EmbeddingModel, String), AppError> {
        let pipeline_configs = self
            .pipeline_configuration_repository
            .load_all()
            .await
            .map_err(|e: PipelineConfigurationRepositoryError| {
                AppError::Internal(format!("load pipeline configs: {e}"))
            })?;

        let pc = pipeline_configs
            .iter()
            .find(|pc| pc.pipeline_configuration_id == pipeline_configuration_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "pipeline configuration {pipeline_configuration_id} not found"
                ))
            })?;

        let catalog = self.configuration_repository.load().await.map_err(
            |e: ConfigurationRepositoryError| {
                AppError::Internal(format!("load configuration: {e}"))
            },
        )?;

        let embedding_model = catalog
            .embedding_models
            .iter()
            .find(|m| m.embedding_model_id == pc.embedding_model_id)
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "embedding model {} not found in configuration",
                    pc.embedding_model_id
                ))
            })?
            .clone();

        let vector_index_name = catalog
            .vector_indexes
            .iter()
            .find(|i| i.index_id == pc.vector_index_id)
            .map(|i| i.name.clone())
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "vector index {} not found in configuration",
                    pc.vector_index_id
                ))
            })?;

        Ok((embedding_model, vector_index_name))
    }
}
