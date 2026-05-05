mod chunking;
mod embedding;
mod evaluation;
mod ingest;
mod post;
pub(crate) mod serde_compat;
mod settings;
mod vector;

pub use chunking::{ChunkStrategy, ChunkingConfig};
pub use embedding::{
    catalog_for_backend, CatalogEntry, EmbedResult, EmbedderBackend, EmbeddingModel,
    CLOUDFLARE_EMBEDDING_MODELS, OLLAMA_EMBEDDING_MODELS,
};
pub use evaluation::{
    ChunkingVariant, EvaluationDataset, EvaluationDatasetStatus, EvaluationGenerationBackend,
    EvaluationJobInfo, EvaluationMetrics, EvaluationQuestion, EvaluationQuestionResult,
    EvaluationReference, EvaluationRunOptions, EvaluationRunResult, EvaluationSettings,
    EvaluationVariantResult,
};
pub use ingest::{IngestJobInfo, IngestOptions, LogEvent, LogLevel};
pub use post::{ChunkPreview, GlossaryTermDto, PostDetailDto, PostSummary};
pub use settings::SettingsDto;
pub use vector::{VectorIndexConfig, VectorProvider};
