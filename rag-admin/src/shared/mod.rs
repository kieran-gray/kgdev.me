mod chunking;
mod configuration_commands;
mod pipeline_configuration;
mod embedding;
mod evaluation;
mod ingest;
mod post;
pub(crate) mod serde_compat;
mod settings;
mod vector;

pub use chunking::{
    BertChunkingConfig, ChunkParamDefinition, ChunkParamKey, ChunkStrategy, ChunkerDefinition,
    ChunkingConfig, LlmChunkingConfig, SectionChunkingConfig,
};
pub use configuration_commands::{
    AddEmbeddingModelDto, AddGenerationModelDto, AddProviderDto, AddVectorIndexDto,
    ConfigurationCommandDto, ProviderType, RemoveAiProviderDto, RemoveEmbeddingModelDto,
    RemoveGenerationModelDto, RemoveVectorIndexDto, RemoveVectorStoreProviderDto,
    SetCurrentEmbeddingModelDto, SetCurrentGenerationModelDto, SetCurrentVectorIndexDto,
    UpdateAiProviderDto, UpdateEmbeddingModelDto, UpdateGenerationModelDto, UpdateVectorIndexDto,
    UpdateVectorStoreProviderDto,
};
pub use pipeline_configuration::{
    AiProviderDto, EmbeddingModelDto, GenerationModelDto, PipelineConfigurationDto,
    VectorIndexDto, VectorStoreProviderDto,
};
pub use embedding::{
    catalog_for_backend, CatalogEntry, EmbedResult, EmbedderBackend, EmbeddingModel,
    CLOUDFLARE_EMBEDDING_MODELS, OLLAMA_EMBEDDING_MODELS,
};
pub use evaluation::{
    evaluation_score, ordered_f32_vec, plain_f32_vec, ChunkingVariant, EvaluationAutotuneRequest,
    EvaluationAutotuneSummary, EvaluationDataset, EvaluationDatasetStatus,
    EvaluationGenerationBackend, EvaluationJobInfo, EvaluationMetrics, EvaluationQuestion,
    EvaluationQuestionResult, EvaluationReference, EvaluationReferenceResult,
    EvaluationResultSplit, EvaluationRunOptions, EvaluationRunResult, EvaluationRunSummary,
    EvaluationScorePolicy, EvaluationScoreWeights, EvaluationSettings, EvaluationVariantResult,
};
pub use ingest::{IngestJobInfo, IngestOptions, LogEvent, LogLevel};
pub use post::{ChunkPreview, GlossaryTermDto, PostDetailDto, PostSummary};
pub use settings::SettingsDto;
pub use vector::{VectorIndexConfig, VectorProvider};
