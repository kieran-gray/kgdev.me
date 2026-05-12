use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::sync::Notify;

use crate::server::application::chunking::chunkers::{
    register_builtin_chunkers, BuiltinChunkerDeps,
};
use crate::server::application::chunking::ChunkerRegistry;
use crate::server::application::configuration::ports::EvaluationDefaultsStore;
use crate::server::application::configuration::{
    ChunkingConfigurationQueryService, ConfigurationCommandHandler, ConfigurationQueryService,
    PipelineConfigurationQueryService, PipelineResolver, SweepTemplateQueryService,
};
use crate::server::application::embedding::ports::Embedder;
use crate::server::application::embedding::EmbeddingService;
use crate::server::application::evaluation::effects::{
    EvaluationDatasetEffect, EvaluationDatasetEffectExecutor, EvaluationRunEffect,
    EvaluationRunEffectExecutor,
};
use crate::server::application::evaluation::ports::{EvaluationGenerator, Retriever};
use crate::server::application::evaluation::query_service::EvaluationQueryService;
use crate::server::application::indexing::ports::VectorIndex;
use crate::server::application::indexing::VectorIndexResolver;
use crate::server::application::indexing::{
    IndexingCommandHandler, IndexingEffect, IndexingEffectExecutor,
};
use crate::server::application::llm::ChatService;
use crate::server::application::ports::{ChatClient, Clock, IdGenerator, MarkdownParser};
use crate::server::application::query::QueryService;
use crate::server::application::source_document::ports::{BlobStore, PostChunkingConfigStore};
use crate::server::application::source_document::ports::{
    SourceAdapterRegistry, VectorIndexProvider,
};
use crate::server::application::source_document::{
    SourceDocumentCommandHandler, SourceDocumentIngestService, SourceDocumentIngestServiceDeps,
    SourceDocumentQueryService,
};
use crate::server::application::{
    spawn_activity_projection, ActivityRegistry, AppError, JobRegistry,
};
use crate::server::domain::chunk_set::repository::ChunkSetRepository;
use crate::server::domain::configuration::aggregate::Configuration;
use crate::server::domain::configuration::chunking_configuration::{
    ChunkingConfigurationProjector, ChunkingConfigurationRepository,
};
use crate::server::domain::configuration::kinds::{AiProviderKind, VectorStoreKind};
use crate::server::domain::configuration::pipeline_configuration::{
    PipelineConfigurationProjector, PipelineConfigurationRepository,
};
use crate::server::domain::configuration::projector::ConfigurationProjector;
use crate::server::domain::configuration::sweep_template::{
    SweepTemplateProjector, SweepTemplateRepository,
};
use crate::server::domain::configuration::ConfigurationRepository;
use crate::server::domain::embedding_set::repository::EmbeddingSetRepository;
use crate::server::domain::evaluation::dataset::aggregate::EvaluationDataset;
use crate::server::domain::evaluation::dataset::policies::derive_dataset_effects;
use crate::server::domain::evaluation::dataset::projector::EvaluationDatasetProjector;
use crate::server::domain::evaluation::dataset::repository::EvaluationDatasetRepository;
use crate::server::domain::evaluation::run::aggregate::EvaluationRun;
use crate::server::domain::evaluation::run::policies::derive_run_effects;
use crate::server::domain::evaluation::run::projector::EvaluationRunProjector;
use crate::server::domain::evaluation::run::repository::EvaluationRunRepository;
use crate::server::domain::indexing::aggregate::Indexing;
use crate::server::domain::indexing::policies::derive_indexing_effects;
use crate::server::domain::indexing::projector::IndexingProjector;
use crate::server::domain::indexing::repository::IndexingRepository;
use crate::server::domain::source_document::aggregate::SourceDocument;
use crate::server::domain::source_document::projector::SourceDocumentProjector;
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
use crate::server::event_sourcing::Aggregate;
use crate::server::infrastructure::chunking::FilePostChunkingConfigStore;
use crate::server::infrastructure::clients::{CloudflareApi, OllamaApi};
use crate::server::infrastructure::configuration::{
    FileEvaluationDefaultsStore, PostgresChunkingConfigurationRepository,
    PostgresConfigurationRepository, PostgresPipelineConfigurationRepository,
    PostgresSweepTemplateRepository,
};
use crate::server::infrastructure::embedding::{OllamaEmbedder, WorkersAiEmbedder};
use crate::server::infrastructure::evaluation::{
    ChatBasedEvaluationGenerator, PgvectorRetriever, PostgresEvaluationDatasetRepository,
    PostgresEvaluationRunRepository,
};
use crate::server::infrastructure::event_sourcing::{
    spawn_postgres_event_listener, PostgresAggregateSnapshotStore, PostgresCheckpointRepository,
    PostgresEffectLedger, PostgresEventStore,
};
use crate::server::infrastructure::http_client::ReqwestHttpClient;
use crate::server::infrastructure::id::UuidGenerator;
use crate::server::infrastructure::indexing::PostgresIndexingRepository;
use crate::server::infrastructure::llm::OllamaChatClient;
use crate::server::infrastructure::markdown::MarkdownRsParser;
use crate::server::infrastructure::source_document::{
    HttpBlogAdapter, PostgresBlobStore, PostgresChunkSetRepository, PostgresEmbeddingSetRepository,
    PostgresSourceDocumentRepository,
};
use crate::server::infrastructure::time::SystemClock;
use crate::server::infrastructure::tokenizer::HuggingFaceTokenizer;
use crate::server::infrastructure::vector::{
    CloudflareVectorIndexProvider, PostgresVectorIndexProvider,
};
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
    pub sweep_template_query_service: Arc<SweepTemplateQueryService>,
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
    pub activity_registry: Arc<ActivityRegistry>,
    pub post_chunking_config_store: Arc<dyn PostChunkingConfigStore>,
    pub source_document_ingest_service: Arc<SourceDocumentIngestService>,
    pub source_document_query_service: Arc<SourceDocumentQueryService>,
    pub query_service: Arc<QueryService>,
    pub source_adapter_registry: Arc<SourceAdapterRegistry>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub async fn initialize() -> Result<Self, SetupError> {
        // ---- Configuration & infrastructure ----
        let config = Config::from_env()?;
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let id_generator: Arc<dyn IdGenerator> = Arc::new(UuidGenerator);
        let pool = Self::connect_database(&config.database_url).await?;

        let http = Arc::new(
            ReqwestHttpClient::new()
                .map_err(|e| SetupError::Internal(format!("http client: {e}")))?,
        );
        let cf_api = Arc::new(CloudflareApi::new(
            Arc::clone(&http),
            config.cloudflare.clone(),
        ));
        let ollama_api = Arc::new(OllamaApi::new(
            Arc::clone(&http),
            config.ollama.base_url.clone(),
        ));

        // ---- Shared event-sourcing infrastructure ----
        let event_bus = Arc::new(EventBus::new());
        let checkpoint_repository: Arc<dyn CheckpointRepository> =
            Arc::new(PostgresCheckpointRepository::new(pool.clone()));
        let mut wakeups: HashMap<String, Arc<Notify>> = HashMap::new();

        // ---- Aggregate wirings (write side) ----
        let configuration_wiring = build_aggregate_wiring::<Configuration>(&pool);
        let source_document_wiring = build_aggregate_wiring::<SourceDocument>(&pool);
        let indexing_wiring = build_aggregate_wiring::<Indexing>(&pool);
        let dataset_wiring = build_aggregate_wiring::<EvaluationDataset>(&pool);
        let run_wiring = build_aggregate_wiring::<EvaluationRun>(&pool);

        // ---- Read-side repositories ----
        let configuration_repository: Arc<dyn ConfigurationRepository> =
            Arc::new(PostgresConfigurationRepository::new(pool.clone()));
        let pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository> =
            Arc::new(PostgresPipelineConfigurationRepository::new(pool.clone()));
        let chunking_configuration_repository: Arc<dyn ChunkingConfigurationRepository> =
            Arc::new(PostgresChunkingConfigurationRepository::new(pool.clone()));
        let sweep_template_repository: Arc<dyn SweepTemplateRepository> =
            Arc::new(PostgresSweepTemplateRepository::new(pool.clone()));
        let source_document_repository: Arc<dyn SourceDocumentRepository> =
            Arc::new(PostgresSourceDocumentRepository::new(pool.clone()));
        let indexing_repository: Arc<dyn IndexingRepository> =
            Arc::new(PostgresIndexingRepository::new(pool.clone()));
        let evaluation_dataset_repository: Arc<dyn EvaluationDatasetRepository> =
            Arc::new(PostgresEvaluationDatasetRepository::new(pool.clone()));
        let evaluation_run_repository: Arc<dyn EvaluationRunRepository> =
            Arc::new(PostgresEvaluationRunRepository::new(pool.clone()));
        let blob_store: Arc<dyn BlobStore> = Arc::new(PostgresBlobStore::new(pool.clone()));
        let chunk_set_repository: Arc<dyn ChunkSetRepository> =
            Arc::new(PostgresChunkSetRepository::new(pool.clone()));
        let embedding_set_repository: Arc<dyn EmbeddingSetRepository> =
            Arc::new(PostgresEmbeddingSetRepository::new(pool.clone()));

        // ---- Domain engines & shared services ----
        let tokenizer = HuggingFaceTokenizer::load_or_fetch(tokenizer_path(), Arc::clone(&http))
            .await
            .map_err(|e| SetupError::Internal(format!("tokenizer: {e}")))?;
        let markdown_parser: Arc<dyn MarkdownParser> = Arc::new(MarkdownRsParser);

        let embedders: HashMap<AiProviderKind, Arc<dyn Embedder>> = HashMap::from([
            (
                AiProviderKind::Cloudflare,
                WorkersAiEmbedder::new(Arc::clone(&cf_api)) as Arc<dyn Embedder>,
            ),
            (
                AiProviderKind::Ollama,
                OllamaEmbedder::new(Arc::clone(&ollama_api)) as Arc<dyn Embedder>,
            ),
        ]);
        let embedding_service = EmbeddingService::new(
            embedders,
            Arc::clone(&configuration_wiring.aggregate_repository),
        );

        let ollama_chat_client: Arc<dyn ChatClient> =
            OllamaChatClient::new(Arc::clone(&http), config.ollama.base_url.clone());
        let chat_clients: HashMap<AiProviderKind, Arc<dyn ChatClient>> = HashMap::from([(
            AiProviderKind::Ollama,
            Arc::clone(&ollama_chat_client) as Arc<dyn ChatClient>,
        )]);
        let chat_service = ChatService::new(
            chat_clients,
            Arc::clone(&configuration_wiring.aggregate_repository),
        );

        let vector_providers: HashMap<VectorStoreKind, Arc<dyn VectorIndexProvider>> =
            HashMap::from([
                (
                    VectorStoreKind::CloudflareVectorize,
                    CloudflareVectorIndexProvider::new(Arc::clone(&cf_api))
                        as Arc<dyn VectorIndexProvider>,
                ),
                (
                    VectorStoreKind::Postgres,
                    PostgresVectorIndexProvider::new(pool.clone()) as Arc<dyn VectorIndexProvider>,
                ),
            ]);
        let vector_index_resolver = VectorIndexResolver::new(
            vector_providers,
            Arc::clone(&configuration_wiring.aggregate_repository),
        );

        let pipeline_resolver = PipelineResolver::new(
            Arc::clone(&pipeline_configuration_repository),
            Arc::clone(&embedding_service),
            Arc::clone(&chat_service),
            Arc::clone(&vector_index_resolver),
        );

        let evaluation_defaults_store: Arc<dyn EvaluationDefaultsStore> =
            FileEvaluationDefaultsStore::new(evaluation_defaults_path());

        let evaluation_generator: Arc<dyn EvaluationGenerator> =
            ChatBasedEvaluationGenerator::new(Arc::clone(&chat_service));

        let chunking_engine = Self::build_chunking_engine(
            tokenizer,
            Arc::clone(&markdown_parser),
            Arc::clone(&ollama_chat_client),
            Arc::clone(&configuration_wiring.aggregate_repository),
        );

        let post_chunking_config_store: Arc<dyn PostChunkingConfigStore> =
            FilePostChunkingConfigStore::new(post_chunking_config_path());
        let job_registry = Arc::new(JobRegistry::new());
        let activity_registry = Arc::new(ActivityRegistry::new());
        spawn_activity_projection(Arc::clone(&activity_registry), Arc::clone(&event_bus));

        // ---- Command handlers (thin wrappers over CommandProcessor) ----
        let configuration_command_handler =
            ConfigurationCommandHandler::new(Arc::clone(&configuration_wiring.command_processor));
        let source_document_command_handler = SourceDocumentCommandHandler::new(Arc::clone(
            &source_document_wiring.command_processor,
        ));
        let indexing_command_handler =
            IndexingCommandHandler::new(Arc::clone(&indexing_wiring.command_processor));

        // ---- Query services ----
        let configuration_query_service =
            ConfigurationQueryService::new(Arc::clone(&configuration_repository));
        let pipeline_configuration_query_service = PipelineConfigurationQueryService::new(
            Arc::clone(&pipeline_configuration_repository),
            Arc::clone(&configuration_repository),
        );
        let chunking_configuration_query_service =
            ChunkingConfigurationQueryService::new(Arc::clone(&chunking_configuration_repository));
        let sweep_template_query_service =
            SweepTemplateQueryService::new(Arc::clone(&sweep_template_repository));
        let evaluation_query_service = EvaluationQueryService::new(
            Arc::clone(&evaluation_dataset_repository),
            Arc::clone(&evaluation_run_repository),
        );

        // ---- Projection drivers (configuration, source_document, indexing have no effects) ----
        spawn_driver::<Configuration, ()>(
            Arc::clone(&configuration_wiring.event_store),
            vec![
                Arc::new(ConfigurationProjector::new(Arc::clone(
                    &configuration_repository,
                ))),
                Arc::new(PipelineConfigurationProjector::new(Arc::clone(
                    &pipeline_configuration_repository,
                ))),
                Arc::new(ChunkingConfigurationProjector::new(Arc::clone(
                    &chunking_configuration_repository,
                ))),
                Arc::new(SweepTemplateProjector::new(Arc::clone(
                    &sweep_template_repository,
                ))),
            ],
            None,
            Arc::clone(&checkpoint_repository),
            Arc::clone(&event_bus),
            &mut wakeups,
        );
        spawn_driver::<SourceDocument, ()>(
            Arc::clone(&source_document_wiring.event_store),
            vec![Arc::new(SourceDocumentProjector::new(Arc::clone(
                &source_document_repository,
            )))],
            None,
            Arc::clone(&checkpoint_repository),
            Arc::clone(&event_bus),
            &mut wakeups,
        );
        // ---- Indexing pipeline (with process manager / effects) ----
        let indexing_effect_ledger: Arc<dyn EffectLedger<IndexingEffect>> =
            Arc::new(PostgresEffectLedger::<IndexingEffect>::new(pool.clone()));

        let indexing_effect_executor = IndexingEffectExecutor::new(
            Arc::clone(&source_document_repository),
            Arc::clone(&indexing_repository),
            Arc::clone(&blob_store),
            Arc::clone(&chunking_engine),
            Arc::clone(&chunk_set_repository),
            Arc::clone(&embedding_service),
            Arc::clone(&embedding_set_repository),
            Arc::clone(&vector_index_resolver),
            Arc::clone(&pipeline_resolver),
            Arc::clone(&indexing_wiring.command_processor),
            Arc::clone(&job_registry),
            Arc::clone(&activity_registry),
            Arc::clone(&clock),
            Arc::clone(&id_generator),
        );

        let indexing_process_manager = Arc::new(ProcessManager::<Indexing, IndexingEffect>::new(
            Arc::clone(&indexing_wiring.aggregate_repository),
            indexing_effect_ledger,
            indexing_effect_executor,
            derive_indexing_effects,
        ));

        spawn_driver::<Indexing, IndexingEffect>(
            Arc::clone(&indexing_wiring.event_store),
            vec![Arc::new(IndexingProjector::new(Arc::clone(
                &indexing_repository,
            )))],
            Some(indexing_process_manager),
            Arc::clone(&checkpoint_repository),
            Arc::clone(&event_bus),
            &mut wakeups,
        );

        // ---- Evaluation pipeline (with process managers / effects) ----
        let dataset_effect_ledger: Arc<dyn EffectLedger<EvaluationDatasetEffect>> = Arc::new(
            PostgresEffectLedger::<EvaluationDatasetEffect>::new(pool.clone()),
        );
        let run_effect_ledger: Arc<dyn EffectLedger<EvaluationRunEffect>> = Arc::new(
            PostgresEffectLedger::<EvaluationRunEffect>::new(pool.clone()),
        );

        let dataset_effect_executor = EvaluationDatasetEffectExecutor::new(
            Arc::clone(&source_document_repository),
            Arc::clone(&blob_store),
            Arc::clone(&evaluation_generator),
            Arc::clone(&embedding_service),
            Arc::clone(&dataset_wiring.command_processor),
            Arc::clone(&job_registry),
            Arc::clone(&activity_registry),
            Arc::clone(&clock),
        );
        let evaluation_retriever: Arc<dyn Retriever> =
            Arc::new(PgvectorRetriever::new(pool.clone()));

        let run_effect_executor = EvaluationRunEffectExecutor::new(
            Arc::clone(&source_document_repository),
            Arc::clone(&blob_store),
            Arc::clone(&chunking_engine),
            Arc::clone(&chunk_set_repository),
            Arc::clone(&embedding_service),
            Arc::clone(&embedding_set_repository),
            Arc::clone(&evaluation_dataset_repository),
            Arc::clone(&evaluation_retriever),
            Arc::clone(&run_wiring.command_processor),
            Arc::clone(&pipeline_resolver),
            Arc::clone(&job_registry),
            Arc::clone(&activity_registry),
            Arc::clone(&clock),
            Arc::clone(&id_generator),
        );

        let dataset_process_manager = Arc::new(ProcessManager::<
            EvaluationDataset,
            EvaluationDatasetEffect,
        >::new(
            Arc::clone(&dataset_wiring.aggregate_repository),
            dataset_effect_ledger,
            dataset_effect_executor,
            derive_dataset_effects,
        ));
        let run_process_manager =
            Arc::new(ProcessManager::<EvaluationRun, EvaluationRunEffect>::new(
                Arc::clone(&run_wiring.aggregate_repository),
                run_effect_ledger,
                run_effect_executor,
                derive_run_effects,
            ));

        spawn_driver::<EvaluationDataset, EvaluationDatasetEffect>(
            Arc::clone(&dataset_wiring.event_store),
            vec![Arc::new(EvaluationDatasetProjector::new(Arc::clone(
                &evaluation_dataset_repository,
            )))],
            Some(dataset_process_manager),
            Arc::clone(&checkpoint_repository),
            Arc::clone(&event_bus),
            &mut wakeups,
        );
        spawn_driver::<EvaluationRun, EvaluationRunEffect>(
            Arc::clone(&run_wiring.event_store),
            vec![Arc::new(EvaluationRunProjector::new(Arc::clone(
                &evaluation_run_repository,
            )))],
            Some(run_process_manager),
            checkpoint_repository,
            Arc::clone(&event_bus),
            &mut wakeups,
        );

        spawn_postgres_event_listener(pool, wakeups);

        // ---- Source document ingest pipeline ----
        let mut source_adapter_registry = SourceAdapterRegistry::new();
        source_adapter_registry.register(HttpBlogAdapter::new(
            Arc::clone(&http),
            config.blog_url.clone(),
        ));
        let source_adapter_registry = Arc::new(source_adapter_registry);

        let source_document_ingest_service =
            SourceDocumentIngestService::new(SourceDocumentIngestServiceDeps {
                source_document_command_handler,
                indexing_command_handler,
                source_document_repository: Arc::clone(&source_document_repository),
                blob_store: Arc::clone(&blob_store),
                source_adapter_registry: Arc::clone(&source_adapter_registry),
                pipeline_resolver: Arc::clone(&pipeline_resolver),
                clock,
                id_generator,
            });
        let source_document_query_service = SourceDocumentQueryService::new(
            Arc::clone(&source_document_repository),
            indexing_repository,
            chunk_set_repository,
            blob_store,
            Arc::clone(&markdown_parser),
        );
        let query_service = QueryService::new(
            Arc::clone(&pipeline_resolver),
            Arc::clone(&embedding_service),
            Arc::clone(&vector_index_resolver),
            source_document_repository,
        );

        let state = Self {
            configuration_command_handler,
            configuration_query_service,
            pipeline_configuration_query_service,
            chunking_configuration_query_service,
            sweep_template_query_service,
            configuration_repository,
            evaluation_dataset_command_processor: dataset_wiring.command_processor,
            evaluation_run_command_processor: run_wiring.command_processor,
            evaluation_query_service,
            embedding_service,
            chat_service,
            pipeline_resolver,
            vector_index_resolver,
            evaluation_defaults_store,
            job_registry,
            activity_registry,
            post_chunking_config_store,
            source_document_ingest_service,
            source_document_query_service,
            query_service,
            source_adapter_registry,
            event_bus,
        };

        state.run_startup_checks().await;
        Ok(state)
    }

    async fn connect_database(url: &str) -> Result<PgPool, SetupError> {
        let pool = PgPoolOptions::new()
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
        markdown_parser: Arc<dyn MarkdownParser>,
        chat_client: Arc<dyn ChatClient>,
        configuration_repository: Arc<AggregateRepository<Configuration>>,
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

    async fn run_startup_checks(&self) {
        if let Err(e) = seed_if_empty(
            &self.chunking_configuration_query_service,
            &self.sweep_template_query_service,
            &self.configuration_query_service,
            &self.configuration_command_handler,
        )
        .await
        {
            tracing::warn!("configuration seed: {e}");
        }
    }
}

struct AggregateWiring<A: Aggregate> {
    aggregate_repository: Arc<AggregateRepository<A>>,
    event_store: Arc<dyn EventStore<A::Event>>,
    command_processor: Arc<CommandProcessor<A>>,
}

fn build_aggregate_wiring<A>(pool: &PgPool) -> AggregateWiring<A>
where
    A: Aggregate + 'static,
    AppError: From<A::Error>,
{
    let event_store: Arc<dyn EventStore<A::Event>> = Arc::new(PostgresEventStore::<A::Event>::new(
        pool.clone(),
        A::aggregate_type(),
    ));
    let snapshot_store = Arc::new(PostgresAggregateSnapshotStore::<A>::new(pool.clone()));
    let aggregate_repository = Arc::new(AggregateRepository::<A>::new(
        Arc::clone(&event_store),
        snapshot_store,
    ));
    let command_processor = Arc::new(CommandProcessor::new(Arc::clone(&aggregate_repository)));
    AggregateWiring {
        aggregate_repository,
        event_store,
        command_processor,
    }
}

fn spawn_driver<A, R>(
    event_store: Arc<dyn EventStore<A::Event>>,
    projectors: Vec<Arc<dyn Projector<A::Event>>>,
    process_manager: Option<Arc<ProcessManager<A, R>>>,
    checkpoint_repository: Arc<dyn CheckpointRepository>,
    event_bus: Arc<EventBus>,
    wakeups: &mut HashMap<String, Arc<Notify>>,
) where
    A: Aggregate + 'static,
    R: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    AppError: From<A::Error>,
{
    let wakeup = Arc::new(Notify::new());
    let driver = Arc::new(ProjectionDriver::<A, R>::new(
        event_store,
        projectors,
        checkpoint_repository,
        event_bus,
        process_manager,
        Arc::clone(&wakeup),
    ));
    wakeups.insert(A::aggregate_type().to_owned(), wakeup);
    tokio::spawn(driver.run());
}

// Keep VectorIndex in scope for downstream re-exports.
#[allow(dead_code)]
fn _vector_index_imported(_: Arc<dyn VectorIndex>) {}
