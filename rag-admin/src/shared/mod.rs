mod chunking;
mod embedding;
mod ingest;
mod post;
mod settings;
mod vector;

pub use chunking::{ChunkStrategy, ChunkingConfig};
pub use embedding::{
    catalog_for_backend, CatalogEntry, EmbedResult, EmbedderBackend, EmbeddingModel,
    CLOUDFLARE_EMBEDDING_MODELS, OLLAMA_EMBEDDING_MODELS,
};
pub use ingest::{IngestJobInfo, IngestOptions, LogEvent, LogLevel};
pub use post::{ChunkPreview, GlossaryTermDto, PostDetailDto, PostSummary};
pub use settings::SettingsDto;
pub use vector::{VectorIndexConfig, VectorProvider};
