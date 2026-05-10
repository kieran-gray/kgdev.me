use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Notify, RwLock};

use crate::server::application::blog::ports::PostChunkingConfigStore;
use crate::server::application::chunking::chunkers::{
    register_builtin_chunkers, BuiltinChunkerDeps,
};
use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::configuration::ports::ConfigurationEventStore;
use crate::server::application::configuration::{
    ConfigurationCommandHandler, ConfigurationQueryService, PipelineConfigurationQueryService,
};
use crate::server::application::embedding::ports::Embedder;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::effects::{
    EvaluationDatasetEffect, EvaluationDatasetEffectExecutor, EvaluationRunEffect,
    EvaluationRunEffectExecutor,
};
use crate::server::application::evaluation::ports::EvaluationGenerator;
use crate::server::application::evaluation::projectors::{
    EvaluationDatasetProjector, EvaluationRunProjector,
};
use crate::server::application::evaluation::query_service::EvaluationQueryService;
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
use crate::server::domain::evaluation::dataset::aggregate::{
    self as dataset_aggregate, EvaluationDataset,
};
use crate::server::domain::evaluation::dataset::events::EvaluationDatasetEvent;
use crate::server::domain::evaluation::dataset::policies::derive_dataset_effects;
use crate::server::domain::evaluation::dataset::repository::EvaluationDatasetRepository;
use crate::server::domain::evaluation::run::aggregate::{self as run_aggregate, EvaluationRun};
use crate::server::domain::evaluation::run::events::EvaluationRunEvent;
use crate::server::domain::evaluation::run::policies::derive_run_effects;
use crate::server::domain::evaluation::run::repository::EvaluationRunRepository;
use crate::server::domain::indexing::repository::IndexingRepository;
use crate::server::domain::source_document::repository::SourceDocumentRepository;
use crate::server::event_sourcing::aggregate_repository::AggregateRepository;
use crate::server::event_sourcing::checkpoint::CheckpointRepository;
use crate::server::event_sourcing::command_processor::CommandProcessor;
use crate::server::event_sourcing::effect::EffectLedger;
use crate::server::event_sourcing::event_bus::EventBus;
use crate::server::event_sourcing::event_store::EventStore;
use crate::server::event_sourcing::process_manager::ProcessManager;
use crate::server::event_sourcing::projection_driver::ProjectionDriver;
use crate::server::event_sourcing::projector::Projector;
use crate::server::infrastructure::blog::HttpBlogSource;
use crate::server::infrastructure::chunking::FilePostChunkingConfigStore;
use crate::server::infrastructure::clients::{CloudflareApi, OllamaApi};
use crate::server::infrastructure::configuration::{
    PostgresConfigurationRepository, PostgresPipelineConfigurationRepository,
};
use crate::server::infrastructure::embedding::{OllamaEmbedder, WorkersAiEmbedder};
use crate::server::infrastructure::evaluation::{
    OllamaEvaluationGenerator, PostgresEvaluationDatasetRepository, PostgresEvaluationRunRepository,
};
use crate::server::infrastructure::event_sourcing::{
    spawn_postgres_event_listener, PostgresAggregateSnapshotStore, PostgresCheckpointRepository,
    PostgresEffectLedger, PostgresEventStore as GenericPostgresEventStore,
};
use crate::server::infrastructure::http_client::ReqwestHttpClient;
use crate::server::infrastructure::id::UuidGenerator;
use crate::server::infrastructure::indexing::PostgresIndexingRepository;
use crate::server::infrastructure::llm::OllamaChatClient;
use crate::server::infrastructure::markdown::MarkdownRsParser;
use crate::server::infrastructure::postgres::PostgresEventStore as LegacyPostgresEventStore;
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
    load_settings, post_chunking_config_path, save_settings, settings_path, tokenizer_path,
};
use crate::server::setup::validation;
use crate::shared::{EmbedderBackend, SettingsDto};

pub struct AppState {
    pub settings: Arc<RwLock<SettingsDto>>,
    pub configuration_command_handler: Arc<ConfigurationCommandHandler>,
    pub configuration_query_service: Arc<ConfigurationQueryService>,
    pub pipeline_configuration_query_service: Arc<PipelineConfigurationQueryService>,
    pub evaluation_dataset_command_processor: Arc<CommandProcessor<EvaluationDataset>>,
    pub evaluation_run_command_processor: Arc<CommandProcessor<EvaluationRun>>,
    pub evaluation_query_service: Arc<EvaluationQueryService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub job_registry: Arc<JobRegistry>,
    pub vector_store: Arc<dyn VectorIndex>,
    pub embedder: Arc<dyn Embedder>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    pub source_document_ingest_service: Arc<SourceDocumentIngestService>,
    pub source_document_query_service: Arc<SourceDocumentQueryService>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
    pub event_bus: Arc<EventBus>,
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

        let clock = Arc::new(SystemClock);
        let id_generator = Arc::new(UuidGenerator);

        let configuration_event_store: Arc<dyn ConfigurationEventStore> = Arc::new(
            LegacyPostgresEventStore::new(pool.clone(), "configuration"),
        );
        let configuration_repository: Arc<dyn ConfigurationRepository> =
            Arc::new(PostgresConfigurationRepository::new(pool.clone()));
        let pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository> =
            Arc::new(PostgresPipelineConfigurationRepository::new(pool.clone()));

        let source_document_event_store: Arc<dyn SourceDocumentEventStore> = Arc::new(
            LegacyPostgresEventStore::new(pool.clone(), "source_document"),
        );
        let indexing_event_store: Arc<dyn IndexingEventStore> = Arc::new(
            LegacyPostgresEventStore::new(pool.clone(), "indexing"),
        );
        let source_document_repository: Arc<dyn SourceDocumentRepository> =
            Arc::new(PostgresSourceDocumentRepository::new(pool.clone()));
        let indexing_repository: Arc<dyn IndexingRepository> =
            Arc::new(PostgresIndexingRepository::new(pool.clone()));
        let blob_store = Arc::new(PostgresBlobStore::new(pool.clone()));
        let chunk_set_repository = Arc::new(PostgresChunkSetRepository::new(pool.clone()));
        let embedding_set_repository = Arc::new(PostgresEmbeddingSetRepository::new(pool.clone()));
        let vector_index_factory = CloudflareVectorIndexFactory::new(cf_api.clone());

        let evaluation_dataset_repository: Arc<dyn EvaluationDatasetRepository> =
            Arc::new(PostgresEvaluationDatasetRepository::new(pool.clone()));
        let evaluation_run_repository: Arc<dyn EvaluationRunRepository> =
            Arc::new(PostgresEvaluationRunRepository::new(pool.clone()));

        // -- event_sourcing wiring for evaluation aggregates --

        let event_bus = Arc::new(EventBus::new());

        let dataset_event_store: Arc<dyn EventStore<EvaluationDatasetEvent>> = Arc::new(
            GenericPostgresEventStore::<EvaluationDatasetEvent>::new(
                pool.clone(),
                dataset_aggregate::AGGREGATE_TYPE,
            ),
        );
        let dataset_snapshot_store = Arc::new(PostgresAggregateSnapshotStore::<
            EvaluationDataset,
        >::new(pool.clone()));
        let dataset_aggregate_repository = Arc::new(AggregateRepository::new(
            dataset_event_store.clone(),
            dataset_snapshot_store,
        ));
        let evaluation_dataset_command_processor = Arc::new(CommandProcessor::new(
            dataset_aggregate_repository.clone(),
        ));

        let run_event_store: Arc<dyn EventStore<EvaluationRunEvent>> =
            Arc::new(GenericPostgresEventStore::<EvaluationRunEvent>::new(
                pool.clone(),
                run_aggregate::AGGREGATE_TYPE,
            ));
        let run_snapshot_store =
            Arc::new(PostgresAggregateSnapshotStore::<EvaluationRun>::new(pool.clone()));
        let run_aggregate_repository = Arc::new(AggregateRepository::new(
            run_event_store.clone(),
            run_snapshot_store,
        ));
        let evaluation_run_command_processor =
            Arc::new(CommandProcessor::new(run_aggregate_repository.clone()));

        let checkpoint_repository: Arc<dyn CheckpointRepository> =
            Arc::new(PostgresCheckpointRepository::new(pool.clone()));

        let dataset_effect_ledger: Arc<dyn EffectLedger<EvaluationDatasetEffect>> =
            Arc::new(PostgresEffectLedger::<EvaluationDatasetEffect>::new(pool.clone()));
        let run_effect_ledger: Arc<dyn EffectLedger<EvaluationRunEffect>> =
            Arc::new(PostgresEffectLedger::<EvaluationRunEffect>::new(pool.clone()));

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

        let evaluation_query_service = EvaluationQueryService::new(
            evaluation_dataset_repository.clone(),
            evaluation_run_repository.clone(),
        );

        // -- effect executors (depend on command processors so we can chain commands) --

        let dataset_effect_executor = EvaluationDatasetEffectExecutor::new(
            source_document_repository.clone(),
            blob_store.clone(),
            evaluation_generator.clone(),
            embedding_service.clone(),
            evaluation_dataset_command_processor.clone(),
            settings.clone(),
            clock.clone(),
        );

        let run_effect_executor = EvaluationRunEffectExecutor::new(
            source_document_repository.clone(),
            blob_store.clone(),
            chunking_engine.clone(),
            chunk_set_repository.clone(),
            embedding_service.clone(),
            embedding_set_repository.clone(),
            evaluation_dataset_repository.clone(),
            evaluation_run_command_processor.clone(),
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
            clock.clone(),
            id_generator.clone(),
        );

        // -- process managers (one per aggregate type) --

        let dataset_process_manager = Arc::new(ProcessManager::<
            EvaluationDataset,
            EvaluationDatasetEffect,
        >::new(
            dataset_aggregate_repository.clone(),
            dataset_effect_ledger,
            dataset_effect_executor,
            derive_dataset_effects,
        ));

        let run_process_manager = Arc::new(ProcessManager::<EvaluationRun, EvaluationRunEffect>::new(
            run_aggregate_repository.clone(),
            run_effect_ledger,
            run_effect_executor,
            derive_run_effects,
        ));

        // -- projectors --

        let dataset_projector: Arc<dyn Projector<EvaluationDatasetEvent>> = Arc::new(
            EvaluationDatasetProjector::new(evaluation_dataset_repository.clone()),
        );
        let run_projector: Arc<dyn Projector<EvaluationRunEvent>> = Arc::new(
            EvaluationRunProjector::new(evaluation_run_repository.clone()),
        );

        // -- projection drivers + LISTEN bridge --

        let dataset_wakeup = Arc::new(Notify::new());
        let run_wakeup = Arc::new(Notify::new());

        let dataset_driver = Arc::new(ProjectionDriver::<
            EvaluationDataset,
            EvaluationDatasetEffect,
        >::new(
            dataset_event_store.clone(),
            vec![dataset_projector],
            checkpoint_repository.clone(),
            event_bus.clone(),
            Some(dataset_process_manager.clone()),
            dataset_wakeup.clone(),
        ));
        let run_driver = Arc::new(ProjectionDriver::<EvaluationRun, EvaluationRunEffect>::new(
            run_event_store.clone(),
            vec![run_projector],
            checkpoint_repository.clone(),
            event_bus.clone(),
            Some(run_process_manager.clone()),
            run_wakeup.clone(),
        ));

        let mut wakeups: HashMap<String, Arc<Notify>> = HashMap::new();
        wakeups.insert(dataset_aggregate::AGGREGATE_TYPE.to_string(), dataset_wakeup);
        wakeups.insert(run_aggregate::AGGREGATE_TYPE.to_string(), run_wakeup);
        spawn_postgres_event_listener(pool.clone(), wakeups);

        tokio::spawn(dataset_driver.run());
        tokio::spawn(run_driver.run());

        let mut source_adapter_registry = SourceAdapterRegistry::new();
        source_adapter_registry.register(HttpBlogAdapter::new(blog_source.clone()));
        let source_adapter_registry = Arc::new(source_adapter_registry);

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

        let _ = tokenizer; // tokenizer was previously held by the deleted ChunkingEvaluationService

        let state = Self {
            settings,
            configuration_command_handler,
            configuration_query_service,
            pipeline_configuration_query_service,
            evaluation_dataset_command_processor,
            evaluation_run_command_processor,
            evaluation_query_service,
            embedding_service,
            job_registry,
            vector_store,
            embedder,
            post_chunking_config_store,
            source_document_ingest_service,
            source_document_query_service,
            source_adapter_registry,
            event_bus,
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
