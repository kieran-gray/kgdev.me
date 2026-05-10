use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::server::application::blog::ports::PostChunkingConfigStore;
use crate::server::application::chunking::chunkers::{
    register_builtin_chunkers, BuiltinChunkerDeps,
};
use crate::server::application::chunking::{ChunkerRegistry, PostChunkingService};
use crate::server::application::configuration::{
    ports::ConfigurationEventStore, ConfigurationCommandHandler, ConfigurationQueryService,
    PipelineConfigurationQueryService,
};
use crate::server::application::embedding::{ports::Embedder, EmbeddingService};
use crate::server::application::evaluation::jobs::EvaluationJobService;
use crate::server::application::evaluation::{
    ports::EvaluationGenerator, ChunkingEvaluationService, ChunkingEvaluationServiceDeps,
};
use crate::server::application::indexing::ports::IndexingEventStore;
use crate::server::application::indexing::IndexingCommandHandler;
use crate::server::application::ingest::ports::VectorIndex;
use crate::server::application::source_document::ports::{
    SourceAdapterRegistry, SourceDocumentEventStore,
};
use crate::server::application::source_document::{
    SourceDocumentCommandHandler, SourceDocumentIngestService, SourceDocumentIngestServiceDeps,
    SourceDocumentQueryService,
};
use crate::server::application::{AppError, JobRegistry};
use crate::server::domain::configuration::pipeline_configuration::PipelineConfigurationRepository;
use crate::server::domain::configuration::ConfigurationRepository;
use crate::server::domain::indexing::repository::IndexingRepository;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::infrastructure::blog::HttpBlogSource;
use crate::server::infrastructure::chunking::FilePostChunkingConfigStore;
use crate::server::infrastructure::clients::{CloudflareApi, OllamaApi};
use crate::server::infrastructure::configuration::{
    PostgresConfigurationRepository, PostgresPipelineConfigurationRepository,
};
use crate::server::infrastructure::embedding::{OllamaEmbedder, WorkersAiEmbedder};
use crate::server::infrastructure::evaluation::{
    FileEvaluationDatasetStore, FileEvaluationResultStore, OllamaEvaluationGenerator,
};
use crate::server::infrastructure::http_client::ReqwestHttpClient;
use crate::server::infrastructure::id::UuidGenerator;
use crate::server::infrastructure::indexing::PostgresIndexingRepository;
use crate::server::infrastructure::llm::OllamaChatClient;
use crate::server::infrastructure::markdown::MarkdownRsParser;
use crate::server::infrastructure::postgres::PostgresEventStore;
use crate::server::infrastructure::source_document::{
    HttpBlogAdapter, PostgresBlobStore, PostgresChunkSetRepository, PostgresEmbeddingSetRepository,
    PostgresSourceDocumentRepository,
};
use crate::server::infrastructure::time::SystemClock;
use crate::server::infrastructure::tokenizer::HuggingFaceTokenizer;
use crate::server::infrastructure::vector::{CloudflareVectorIndexFactory, VectorizeVectorIndex};
use crate::server::setup::config::Config;
use crate::server::setup::exceptions::SetupError;
use crate::server::setup::settings::{
    evaluations_dir, load_settings, post_chunking_config_path, save_settings, settings_path,
    tokenizer_path,
};
use crate::server::setup::validation;
use crate::shared::{EmbedderBackend, SettingsDto};

pub struct AppState {
    pub settings: Arc<RwLock<SettingsDto>>,
    pub configuration_command_handler: Arc<ConfigurationCommandHandler>,
    pub configuration_query_service: Arc<ConfigurationQueryService>,
    pub pipeline_configuration_query_service: Arc<PipelineConfigurationQueryService>,
    pub chunking_evaluation_service: Arc<ChunkingEvaluationService>,
    pub evaluation_job_service: Arc<EvaluationJobService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub job_registry: Arc<JobRegistry>,
    pub vector_store: Arc<dyn VectorIndex>,
    pub embedder: Arc<dyn Embedder>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    pub source_document_ingest_service: Arc<SourceDocumentIngestService>,
    pub source_document_query_service: Arc<SourceDocumentQueryService>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
}

impl AppState {
    pub async fn initialize() -> Result<Self, SetupError> {
        let config = Config::from_env()?;

        let settings = load_settings(&settings_path()).await?;
        let settings = Arc::new(RwLock::new(settings));

        let http = Arc::new(
            ReqwestHttpClient::new()
                .map_err(|e| SetupError::Internal(format!("http client: {e}")))?,
        );

        let blog_source = HttpBlogSource::new(http.clone(), config.blog_url.clone());

        let cf_api = Arc::new(CloudflareApi::new(http.clone(), config.cloudflare.clone()));
        let ollama_api = Arc::new(OllamaApi::new(http.clone(), config.ollama.base_url.clone()));

        let embedder: Arc<dyn Embedder> = Arc::new(BackendEmbedder {
            cloudflare: WorkersAiEmbedder::new(cf_api.clone()),
            ollama: OllamaEmbedder::new(ollama_api, settings.clone()),
            settings: settings.clone(),
        });

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await
            .map_err(|e| SetupError::Internal(format!("postgres pool: {e}")))?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| SetupError::Internal(format!("migrations: {e}")))?;

        let configuration_event_store: Arc<dyn ConfigurationEventStore> =
            Arc::new(PostgresEventStore::new(pool.clone(), "configuration"));
        let configuration_repository: Arc<dyn ConfigurationRepository> =
            Arc::new(PostgresConfigurationRepository::new(pool.clone()));
        let pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository> =
            Arc::new(PostgresPipelineConfigurationRepository::new(pool.clone()));

        let source_document_event_store: Arc<dyn SourceDocumentEventStore> =
            Arc::new(PostgresEventStore::new(pool.clone(), "source_document"));
        let indexing_event_store: Arc<dyn IndexingEventStore> =
            Arc::new(PostgresEventStore::new(pool.clone(), "indexing"));
        let source_document_repository: Arc<dyn SourceDocumentRepository> =
            Arc::new(PostgresSourceDocumentRepository::new(pool.clone()));
        let indexing_repository: Arc<dyn IndexingRepository> =
            Arc::new(PostgresIndexingRepository::new(pool.clone()));
        let blob_store = Arc::new(PostgresBlobStore::new(pool.clone()));
        let chunk_set_repository = Arc::new(PostgresChunkSetRepository::new(pool.clone()));
        let embedding_set_repository = Arc::new(PostgresEmbeddingSetRepository::new(pool.clone()));
        let vector_index_factory = CloudflareVectorIndexFactory::new(cf_api.clone());

        let vector_store: Arc<dyn VectorIndex> = VectorizeVectorIndex::new(
            cf_api.clone(),
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
        );
        let chat_client = OllamaChatClient::new(http.clone(), config.ollama.base_url.clone());
        let evaluation_generator: Arc<dyn EvaluationGenerator> =
            OllamaEvaluationGenerator::new(chat_client.clone());
        let markdown_parser = Arc::new(MarkdownRsParser);

        let post_chunking_config_store: Arc<dyn PostChunkingConfigStore> =
            FilePostChunkingConfigStore::new(post_chunking_config_path());

        let tokenizer = HuggingFaceTokenizer::load_or_fetch(tokenizer_path(), http.clone())
            .await
            .map_err(|e| SetupError::Internal(format!("tokenizer: {e}")))?;
        let job_registry = Arc::new(JobRegistry::new());

        let embedding_service = EmbeddingService::new(embedder.clone());

        let mut chunking_engine = ChunkerRegistry::new(tokenizer.clone(), markdown_parser);
        register_builtin_chunkers(&mut chunking_engine, BuiltinChunkerDeps { chat_client });
        let chunking_engine = Arc::new(chunking_engine);
        let post_chunking_service = PostChunkingService::new(chunking_engine.clone());

        let configuration_command_handler = ConfigurationCommandHandler::new(
            configuration_event_store,
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
        );
        let configuration_query_service =
            ConfigurationQueryService::new(configuration_repository.clone());
        let pipeline_configuration_query_service = PipelineConfigurationQueryService::new(
            pipeline_configuration_repository.clone(),
            configuration_repository.clone(),
        );

        let source_document_command_handler = SourceDocumentCommandHandler::new(
            source_document_event_store,
            source_document_repository.clone(),
        );
        let indexing_command_handler =
            IndexingCommandHandler::new(indexing_event_store, indexing_repository.clone());

        let evaluation_dataset_store = FileEvaluationDatasetStore::new(evaluations_dir());
        let evaluation_result_store = FileEvaluationResultStore::new(evaluations_dir());

        let chunking_evaluation_service =
            ChunkingEvaluationService::new(ChunkingEvaluationServiceDeps {
                blog_source: blog_source.clone(),
                generator: evaluation_generator.clone(),
                embedding_service: embedding_service.clone(),
                settings: settings.clone(),
                evaluation_dataset_store,
                evaluation_result_store,
                post_chunking_service,
                tokenizer,
            });

        let evaluation_job_service =
            EvaluationJobService::new(job_registry.clone(), chunking_evaluation_service.clone());

        let mut source_adapter_registry = SourceAdapterRegistry::new();
        source_adapter_registry.register(HttpBlogAdapter::new(blog_source.clone()));
        let source_adapter_registry = Arc::new(source_adapter_registry);

        let clock = Arc::new(SystemClock);
        let id_generator = Arc::new(UuidGenerator);

        let source_document_ingest_service =
            SourceDocumentIngestService::new(SourceDocumentIngestServiceDeps {
                source_document_command_handler,
                indexing_command_handler,
                source_document_repository: source_document_repository.clone(),
                blob_store,
                chunk_set_repository: chunk_set_repository.clone(),
                embedding_set_repository,
                source_adapter_registry: source_adapter_registry.clone(),
                chunker_registry: chunking_engine,
                embedding_service: embedding_service.clone(),
                vector_index_factory,
                configuration_repository,
                pipeline_configuration_repository,
                job_registry: job_registry.clone(),
                clock,
                id_generator,
            });

        let source_document_query_service = SourceDocumentQueryService::new(
            source_document_repository,
            indexing_repository,
            chunk_set_repository,
        );

        let state = Self {
            settings,
            configuration_command_handler,
            configuration_query_service,
            pipeline_configuration_query_service,
            chunking_evaluation_service,
            evaluation_job_service,
            embedding_service,
            job_registry,
            vector_store,
            embedder,
            post_chunking_config_store,
            source_document_ingest_service,
            source_document_query_service,
            source_adapter_registry,
        };

        if let Err(e) = state.validate_active_settings().await {
            tracing::warn!("settings invariant check: {e}");
        }

        Ok(state)
    }

    pub async fn settings_snapshot(&self) -> SettingsDto {
        self.settings.read().await.clone()
    }

    pub async fn save_settings(&self, new_settings: SettingsDto) -> Result<(), SetupError> {
        validation::validate_local(&new_settings).map_err(SetupError::Config)?;
        let previous = {
            let mut guard = self.settings.write().await;
            std::mem::replace(&mut *guard, new_settings.clone())
        };
        if let Err(e) = save_settings(&settings_path(), &new_settings).await {
            *self.settings.write().await = previous;
            return Err(e);
        }
        Ok(())
    }

    async fn validate_active_settings(&self) -> Result<(), String> {
        let snapshot = self.settings_snapshot().await;
        validation::validate_local(&snapshot)
    }
}

struct BackendEmbedder {
    cloudflare: Arc<dyn Embedder>,
    ollama: Arc<dyn Embedder>,
    settings: Arc<RwLock<SettingsDto>>,
}

#[async_trait]
impl Embedder for BackendEmbedder {
    async fn embed_batch(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        let backend = self.settings.read().await.embedding_model.backend;
        match backend {
            EmbedderBackend::Cloudflare => self.cloudflare.embed_batch(model, texts).await,
            EmbedderBackend::Ollama => self.ollama.embed_batch(model, texts).await,
        }
    }
}
