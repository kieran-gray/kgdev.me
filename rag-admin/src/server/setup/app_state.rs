use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Notify;

use crate::server::application::blog::ports::PostChunkingConfigStore;
use crate::server::application::chunking::chunkers::{
    register_builtin_chunkers, BuiltinChunkerDeps,
};
use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::configuration::ports::{
    ConfigurationEventStore, EvaluationDefaultsStore,
};
use crate::server::application::configuration::{
    ChunkingConfigurationQueryService, ConfigurationCommandHandler, ConfigurationQueryService,
    PipelineConfigurationQueryService, PipelineResolver,
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
use crate::server::application::ingest::VectorIndexResolver;
use crate::server::application::llm::ChatService;
use crate::server::application::ports::ChatClient;
use crate::server::application::source_document::ports::{
    SourceAdapterRegistry, SourceDocumentEventStore, VectorIndexProvider,
};
use crate::server::application::source_document::{
    SourceDocumentCommandHandler, SourceDocumentIngestService, SourceDocumentIngestServiceDeps,
    SourceDocumentQueryService,
};
use crate::server::application::JobRegistry;
use crate::server::domain::configuration::chunking_configuration::ChunkingConfigurationRepository;
use crate::server::domain::configuration::kinds::{AiProviderKind, VectorStoreKind};
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
    FileEvaluationDefaultsStore, PostgresChunkingConfigurationRepository,
    PostgresConfigurationRepository, PostgresPipelineConfigurationRepository,
};
use crate::server::infrastructure::embedding::{OllamaEmbedder, WorkersAiEmbedder};
use crate::server::infrastructure::evaluation::{
    ChatBasedEvaluationGenerator, PostgresEvaluationDatasetRepository,
    PostgresEvaluationRunRepository,
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
use crate::server::infrastructure::vector::CloudflareVectorIndexProvider;
use crate::server::setup::config::Config;
use crate::server::setup::exceptions::SetupError;
use crate::server::setup::paths::{
    evaluation_defaults_path, post_chunking_config_path, tokenizer_path,
};
use crate::server::setup::seed::seed_if_empty;

pub struct AppState {
    pub configuration_command_handler: Arc<ConfigurationCommandHandler>,
    pub configuration_query_service: Arc<ConfigurationQueryService>,
    pub pipeline_configuration_query_service: Arc<PipelineConfigurationQueryService>,
    pub chunking_configuration_query_service: Arc<ChunkingConfigurationQueryService>,
    pub configuration_repository: Arc<dyn ConfigurationRepository>,
    pub evaluation_dataset_command_processor: Arc<CommandProcessor<EvaluationDataset>>,
    pub evaluation_run_command_processor: Arc<CommandProcessor<EvaluationRun>>,
    pub evaluation_query_service: Arc<EvaluationQueryService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub chat_service: Arc<ChatService>,
    pub pipeline_resolver: Arc<PipelineResolver>,
    pub vector_index_resolver: Arc<VectorIndexResolver>,
    pub evaluation_defaults_store: Arc<dyn EvaluationDefaultsStore>,
    pub job_registry: Arc<JobRegistry>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    pub source_document_ingest_service: Arc<SourceDocumentIngestService>,
    pub source_document_query_service: Arc<SourceDocumentQueryService>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub async fn initialize() -> Result<Self, SetupError> {
        // ---- Configuration ----
        let config = Config::from_env()?;

        // ---- Core utilities & database ----
        let clock = Arc::new(SystemClock);
        let id_generator = Arc::new(UuidGenerator);
        let pool = Self::connect_database(&config.database_url).await?;

        // ---- External clients ----
        let http = Arc::new(
            ReqwestHttpClient::new()
                .map_err(|e| SetupError::Internal(format!("http client: {e}")))?,
        );
        let cf_api = Arc::new(CloudflareApi::new(http.clone(), config.cloudflare.clone()));
        let ollama_api = Arc::new(OllamaApi::new(http.clone(), config.ollama.base_url.clone()));
        let blog_source = HttpBlogSource::new(http.clone(), config.blog_url.clone());

        // ---- Repositories & event stores ----
        let configuration_event_store: Arc<dyn ConfigurationEventStore> = Arc::new(
            LegacyPostgresEventStore::new(pool.clone(), "configuration"),
        );
        let configuration_repository: Arc<dyn ConfigurationRepository> =
            Arc::new(PostgresConfigurationRepository::new(pool.clone()));
        let pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository> =
            Arc::new(PostgresPipelineConfigurationRepository::new(pool.clone()));
        let chunking_configuration_repository: Arc<dyn ChunkingConfigurationRepository> =
            Arc::new(PostgresChunkingConfigurationRepository::new(pool.clone()));
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
        let evaluation_dataset_repository: Arc<dyn EvaluationDatasetRepository> =
            Arc::new(PostgresEvaluationDatasetRepository::new(pool.clone()));
        let evaluation_run_repository: Arc<dyn EvaluationRunRepository> =
            Arc::new(PostgresEvaluationRunRepository::new(pool.clone()));

        // ---- Domain engines & shared services ----
        let tokenizer = HuggingFaceTokenizer::load_or_fetch(tokenizer_path(), http.clone())
            .await
            .map_err(|e| SetupError::Internal(format!("tokenizer: {e}")))?;
        let markdown_parser = Arc::new(MarkdownRsParser);

        // Provider-kind keyed maps: one Embedder/ChatClient per `AiProviderKind`,
        // one VectorIndexProvider per `VectorStoreKind`. Dispatch happens in
        // the application-layer services (EmbeddingService / ChatService /
        // VectorIndexResolver) based on what the configuration aggregate
        // says about each registered model/index.
        let embedders: HashMap<AiProviderKind, Arc<dyn Embedder>> = HashMap::from([
            (
                AiProviderKind::Cloudflare,
                WorkersAiEmbedder::new(cf_api.clone()) as Arc<dyn Embedder>,
            ),
            (
                AiProviderKind::Ollama,
                OllamaEmbedder::new(ollama_api.clone()) as Arc<dyn Embedder>,
            ),
        ]);
        let embedding_service =
            EmbeddingService::new(embedders, configuration_repository.clone());

        let ollama_chat_client =
            OllamaChatClient::new(http.clone(), config.ollama.base_url.clone());
        let chat_clients: HashMap<AiProviderKind, Arc<dyn ChatClient>> = HashMap::from([
            // Ollama is the only chat backend wired today. Cloudflare Workers
            // AI chat completion is feasible but unused; add it here when a
            // Cloudflare generation model is actually called.
            (
                AiProviderKind::Ollama,
                ollama_chat_client.clone() as Arc<dyn ChatClient>,
            ),
        ]);
        let chat_service = ChatService::new(chat_clients, configuration_repository.clone());

        let vector_providers: HashMap<VectorStoreKind, Arc<dyn VectorIndexProvider>> =
            HashMap::from([(
                VectorStoreKind::CloudflareVectorize,
                CloudflareVectorIndexProvider::new(cf_api.clone())
                    as Arc<dyn VectorIndexProvider>,
            )]);
        let vector_index_resolver =
            VectorIndexResolver::new(vector_providers, configuration_repository.clone());

        let pipeline_resolver = PipelineResolver::new(
            pipeline_configuration_repository.clone(),
            embedding_service.clone(),
            chat_service.clone(),
            vector_index_resolver.clone(),
        );

        let evaluation_defaults_store: Arc<dyn EvaluationDefaultsStore> =
            FileEvaluationDefaultsStore::new(evaluation_defaults_path());

        let evaluation_generator: Arc<dyn EvaluationGenerator> =
            ChatBasedEvaluationGenerator::new(chat_service.clone());

        let chunking_engine = Self::build_chunking_engine(
            tokenizer,
            markdown_parser,
            ollama_chat_client.clone() as Arc<dyn ChatClient>,
            configuration_repository.clone(),
        );

        let post_chunking_config_store: Arc<dyn PostChunkingConfigStore> =
            FilePostChunkingConfigStore::new(post_chunking_config_path());
        let job_registry = Arc::new(JobRegistry::new());

        // ---- Command handlers & query services ----
        let configuration_command_handler = ConfigurationCommandHandler::new(
            configuration_event_store,
            configuration_repository.clone(),
            pipeline_configuration_repository.clone(),
            chunking_configuration_repository.clone(),
        );
        let configuration_query_service =
            ConfigurationQueryService::new(configuration_repository.clone());
        let pipeline_configuration_query_service = PipelineConfigurationQueryService::new(
            pipeline_configuration_repository.clone(),
            configuration_repository.clone(),
        );
        let chunking_configuration_query_service =
            ChunkingConfigurationQueryService::new(chunking_configuration_repository);
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

        // ---- Evaluation event-sourcing pipeline ----
        let EvaluationPipelineHandles {
            event_bus,
            dataset_command_processor: evaluation_dataset_command_processor,
            run_command_processor: evaluation_run_command_processor,
        } = Self::build_evaluation_pipeline(EvaluationPipelineDeps {
            pool,
            clock: clock.clone(),
            id_generator: id_generator.clone(),
            source_document_repository: source_document_repository.clone(),
            blob_store: blob_store.clone(),
            chunk_set_repository: chunk_set_repository.clone(),
            embedding_set_repository: embedding_set_repository.clone(),
            evaluation_dataset_repository,
            evaluation_run_repository,
            pipeline_resolver: pipeline_resolver.clone(),
            chunking_engine: chunking_engine.clone(),
            embedding_service: embedding_service.clone(),
            evaluation_generator,
        });

        // ---- Source document ingest pipeline ----
        let mut source_adapter_registry = SourceAdapterRegistry::new();
        source_adapter_registry.register(HttpBlogAdapter::new(blog_source));
        let source_adapter_registry = Arc::new(source_adapter_registry);

        let source_document_ingest_service =
            SourceDocumentIngestService::new(SourceDocumentIngestServiceDeps {
                source_document_command_handler,
                indexing_command_handler,
                source_document_repository: source_document_repository.clone(),
                indexing_repository: indexing_repository.clone(),
                blob_store,
                chunk_set_repository: chunk_set_repository.clone(),
                embedding_set_repository,
                source_adapter_registry: source_adapter_registry.clone(),
                chunker_registry: chunking_engine,
                embedding_service: embedding_service.clone(),
                vector_index_resolver: vector_index_resolver.clone(),
                pipeline_resolver: pipeline_resolver.clone(),
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
            configuration_command_handler,
            configuration_query_service,
            pipeline_configuration_query_service,
            chunking_configuration_query_service,
            configuration_repository,
            evaluation_dataset_command_processor,
            evaluation_run_command_processor,
            evaluation_query_service,
            embedding_service,
            chat_service,
            pipeline_resolver,
            vector_index_resolver,
            evaluation_defaults_store,
            job_registry,
            post_chunking_config_store,
            source_document_ingest_service,
            source_document_query_service,
            source_adapter_registry,
            event_bus,
        };

        state.run_startup_checks().await;
        Ok(state)
    }

    async fn connect_database(url: &str) -> Result<PgPool, SetupError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| SetupError::Internal(format!("postgres pool: {e}")))?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| SetupError::Internal(format!("migrations: {e}")))?;
        Ok(pool)
    }

    fn build_chunking_engine(
        tokenizer: Arc<HuggingFaceTokenizer>,
        markdown_parser: Arc<MarkdownRsParser>,
        chat_client: Arc<dyn ChatClient>,
        configuration_repository: Arc<dyn ConfigurationRepository>,
    ) -> Arc<ChunkerRegistry> {
        let mut chunking_engine = ChunkerRegistry::new(tokenizer, markdown_parser);
        register_builtin_chunkers(
            &mut chunking_engine,
            BuiltinChunkerDeps {
                chat_client,
                configuration_repository,
            },
        );
        Arc::new(chunking_engine)
    }

    fn build_evaluation_pipeline(deps: EvaluationPipelineDeps) -> EvaluationPipelineHandles {
        let event_bus = Arc::new(EventBus::new());

        // -- Dataset aggregate wiring --
        let dataset_event_store: Arc<dyn EventStore<EvaluationDatasetEvent>> = Arc::new(
            GenericPostgresEventStore::<EvaluationDatasetEvent>::new(
                deps.pool.clone(),
                dataset_aggregate::AGGREGATE_TYPE,
            ),
        );
        let dataset_snapshot_store = Arc::new(PostgresAggregateSnapshotStore::<
            EvaluationDataset,
        >::new(deps.pool.clone()));
        let dataset_aggregate_repository = Arc::new(AggregateRepository::new(
            dataset_event_store.clone(),
            dataset_snapshot_store,
        ));
        let dataset_command_processor =
            Arc::new(CommandProcessor::new(dataset_aggregate_repository.clone()));

        // -- Run aggregate wiring --
        let run_event_store: Arc<dyn EventStore<EvaluationRunEvent>> =
            Arc::new(GenericPostgresEventStore::<EvaluationRunEvent>::new(
                deps.pool.clone(),
                run_aggregate::AGGREGATE_TYPE,
            ));
        let run_snapshot_store = Arc::new(PostgresAggregateSnapshotStore::<EvaluationRun>::new(
            deps.pool.clone(),
        ));
        let run_aggregate_repository = Arc::new(AggregateRepository::new(
            run_event_store.clone(),
            run_snapshot_store,
        ));
        let run_command_processor =
            Arc::new(CommandProcessor::new(run_aggregate_repository.clone()));

        // -- Shared process-manager infrastructure --
        let checkpoint_repository: Arc<dyn CheckpointRepository> =
            Arc::new(PostgresCheckpointRepository::new(deps.pool.clone()));
        let dataset_effect_ledger: Arc<dyn EffectLedger<EvaluationDatasetEffect>> = Arc::new(
            PostgresEffectLedger::<EvaluationDatasetEffect>::new(deps.pool.clone()),
        );
        let run_effect_ledger: Arc<dyn EffectLedger<EvaluationRunEffect>> = Arc::new(
            PostgresEffectLedger::<EvaluationRunEffect>::new(deps.pool.clone()),
        );

        // -- Effect executors (chain back into the command processors) --
        let dataset_effect_executor = EvaluationDatasetEffectExecutor::new(
            deps.source_document_repository.clone(),
            deps.blob_store.clone(),
            deps.evaluation_generator,
            deps.embedding_service.clone(),
            dataset_command_processor.clone(),
            deps.clock.clone(),
        );
        let run_effect_executor = EvaluationRunEffectExecutor::new(
            deps.source_document_repository,
            deps.blob_store,
            deps.chunking_engine,
            deps.chunk_set_repository,
            deps.embedding_service,
            deps.embedding_set_repository,
            deps.evaluation_dataset_repository.clone(),
            run_command_processor.clone(),
            deps.pipeline_resolver,
            deps.clock,
            deps.id_generator,
        );

        // -- Process managers (one per aggregate type) --
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

        // -- Projectors --
        let dataset_projector: Arc<dyn Projector<EvaluationDatasetEvent>> = Arc::new(
            EvaluationDatasetProjector::new(deps.evaluation_dataset_repository),
        );
        let run_projector: Arc<dyn Projector<EvaluationRunEvent>> = Arc::new(
            EvaluationRunProjector::new(deps.evaluation_run_repository),
        );

        // -- Projection drivers + LISTEN bridge --
        let dataset_wakeup = Arc::new(Notify::new());
        let run_wakeup = Arc::new(Notify::new());

        let dataset_driver = Arc::new(ProjectionDriver::<
            EvaluationDataset,
            EvaluationDatasetEffect,
        >::new(
            dataset_event_store,
            vec![dataset_projector],
            checkpoint_repository.clone(),
            event_bus.clone(),
            Some(dataset_process_manager),
            dataset_wakeup.clone(),
        ));
        let run_driver = Arc::new(ProjectionDriver::<EvaluationRun, EvaluationRunEffect>::new(
            run_event_store,
            vec![run_projector],
            checkpoint_repository,
            event_bus.clone(),
            Some(run_process_manager),
            run_wakeup.clone(),
        ));

        let mut wakeups: HashMap<String, Arc<Notify>> = HashMap::new();
        wakeups.insert(dataset_aggregate::AGGREGATE_TYPE.to_string(), dataset_wakeup);
        wakeups.insert(run_aggregate::AGGREGATE_TYPE.to_string(), run_wakeup);
        spawn_postgres_event_listener(deps.pool, wakeups);

        tokio::spawn(dataset_driver.run());
        tokio::spawn(run_driver.run());

        EvaluationPipelineHandles {
            event_bus,
            dataset_command_processor,
            run_command_processor,
        }
    }

    async fn run_startup_checks(&self) {
        if let Err(e) = seed_if_empty(
            &self.chunking_configuration_query_service,
            &self.configuration_query_service,
            &self.configuration_command_handler,
        )
        .await
        {
            tracing::warn!("chunking configuration seed: {e}");
        }
    }
}

struct EvaluationPipelineDeps {
    pool: PgPool,
    clock: Arc<SystemClock>,
    id_generator: Arc<UuidGenerator>,
    source_document_repository: Arc<dyn SourceDocumentRepository>,
    blob_store: Arc<PostgresBlobStore>,
    chunk_set_repository: Arc<PostgresChunkSetRepository>,
    embedding_set_repository: Arc<PostgresEmbeddingSetRepository>,
    evaluation_dataset_repository: Arc<dyn EvaluationDatasetRepository>,
    evaluation_run_repository: Arc<dyn EvaluationRunRepository>,
    pipeline_resolver: Arc<PipelineResolver>,
    chunking_engine: Arc<ChunkerRegistry>,
    embedding_service: Arc<EmbeddingService>,
    evaluation_generator: Arc<dyn EvaluationGenerator>,
}

struct EvaluationPipelineHandles {
    event_bus: Arc<EventBus>,
    dataset_command_processor: Arc<CommandProcessor<EvaluationDataset>>,
    run_command_processor: Arc<CommandProcessor<EvaluationRun>>,
}

// Suppress unused warning while the legacy `VectorIndex` trait import is kept
// in scope for downstream re-exports.
#[allow(dead_code)]
fn _vector_index_imported(_: Arc<dyn VectorIndex>) {}
