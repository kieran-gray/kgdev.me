use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::server::application::blog::{ports::PostChunkingConfigStore, PostService};
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
use crate::server::application::ingest::{ports::VectorIndex, IngestService, IngestServiceDeps};
use crate::server::application::{AppError, JobRegistry};
use crate::server::domain::configuration::ConfigurationRepository;
use crate::server::domain::pipeline_configuration::PipelineConfigurationRepository;
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
use crate::server::infrastructure::ingest::FileManifestStore;
use crate::server::infrastructure::kv::CloudflareKvStore;
use crate::server::infrastructure::llm::OllamaChatClient;
use crate::server::infrastructure::markdown::MarkdownRsParser;
use crate::server::infrastructure::postgres::PostgresEventStore;
use crate::server::infrastructure::tokenizer::{HuggingFaceTokenizer, EMBEDDING_TOKEN_LIMIT};
use crate::server::infrastructure::vector::{CloudflareVectorRecordMapper, VectorizeVectorIndex};
use crate::server::setup::config::Config;
use crate::server::setup::exceptions::SetupError;
use crate::server::setup::settings::{
    evaluations_dir, load_settings, manifest_path, post_chunking_config_path, save_settings,
    settings_path, tokenizer_path,
};
use crate::server::setup::validation;
use crate::shared::{EmbedderBackend, SettingsDto};

pub struct AppState {
    pub settings: Arc<RwLock<SettingsDto>>,
    pub configuration_command_handler: Arc<ConfigurationCommandHandler>,
    pub configuration_query_service: Arc<ConfigurationQueryService>,
    pub pipeline_configuration_query_service: Arc<PipelineConfigurationQueryService>,
    pub ingest_service: Arc<IngestService>,
    pub post_service: Arc<PostService>,
    pub chunking_evaluation_service: Arc<ChunkingEvaluationService>,
    pub evaluation_job_service: Arc<EvaluationJobService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub job_registry: Arc<JobRegistry>,
    pub vector_store: Arc<dyn VectorIndex>,
    pub embedder: Arc<dyn Embedder>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
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

        let vector_store: Arc<dyn VectorIndex> = VectorizeVectorIndex::new(
            cf_api.clone(),
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
        );
        let vector_record_mapper = Arc::new(CloudflareVectorRecordMapper);
        let kv_store = CloudflareKvStore::new(cf_api.clone());
        let chat_client = OllamaChatClient::new(http.clone(), config.ollama.base_url.clone());
        let evaluation_generator: Arc<dyn EvaluationGenerator> =
            OllamaEvaluationGenerator::new(chat_client.clone());
        let markdown_parser = Arc::new(MarkdownRsParser);

        let manifest_store = FileManifestStore::new(manifest_path());
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
        let post_chunking_service = PostChunkingService::new(chunking_engine);

        let configuration_command_handler = ConfigurationCommandHandler::new(
            configuration_event_store,
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
        );
        let configuration_query_service =
            ConfigurationQueryService::new(configuration_repository.clone());
        let pipeline_configuration_query_service = PipelineConfigurationQueryService::new(
            pipeline_configuration_repository.clone(),
            configuration_repository,
        );

        let ingest_service = IngestService::new(IngestServiceDeps {
            blog_source: blog_source.clone(),
            embedding_service: embedding_service.clone(),
            vector_store: vector_store.clone(),
            vector_record_mapper,
            kv_store,
            manifest_store: manifest_store.clone(),
            post_chunking_config_store: post_chunking_config_store.clone(),
            settings: settings.clone(),
            job_registry: job_registry.clone(),
            post_chunking_service: post_chunking_service.clone(),
        });

        let post_service = PostService::new(
            blog_source.clone(),
            manifest_store,
            post_chunking_config_store.clone(),
            tokenizer.clone(),
            EMBEDDING_TOKEN_LIMIT,
            settings.clone(),
            post_chunking_service.clone(),
        );

        let evaluation_dataset_store = FileEvaluationDatasetStore::new(evaluations_dir());
        let evaluation_result_store = FileEvaluationResultStore::new(evaluations_dir());

        let chunking_evaluation_service =
            ChunkingEvaluationService::new(ChunkingEvaluationServiceDeps {
                blog_source,
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

        let state = Self {
            settings,
            configuration_command_handler,
            configuration_query_service,
            pipeline_configuration_query_service,
            ingest_service,
            post_service,
            chunking_evaluation_service,
            evaluation_job_service,
            embedding_service,
            job_registry,
            vector_store,
            embedder,
            post_chunking_config_store,
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
