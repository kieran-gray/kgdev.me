mod chunking;
mod configuration_commands;
mod embedding;
mod evaluation;
mod ingest;
mod configuration;
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
    ConfigurationCommandDto, CreatePipelineConfigurationDto, DeletePipelineConfigurationDto,
    ProviderType, RemoveAiProviderDto, RemoveEmbeddingModelDto, RemoveGenerationModelDto,
    RemoveVectorIndexDto, RemoveVectorStoreProviderDto, UpdateAiProviderDto,
    UpdateEmbeddingModelDto, UpdateGenerationModelDto, UpdatePipelineConfigurationDto,
    UpdateVectorIndexDto, UpdateVectorStoreProviderDto,
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
pub use configuration::{
    AiProviderDto, ConfigurationDto, EmbeddingModelDto, GenerationModelDto,
    PipelineConfigurationDto, VectorIndexDto, VectorStoreProviderDto,
};
pub use post::{ChunkPreview, GlossaryTermDto, PostDetailDto, PostSummary};
pub use settings::SettingsDto;
pub use vector::{VectorIndexConfig, VectorProvider};
