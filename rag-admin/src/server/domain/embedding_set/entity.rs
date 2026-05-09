use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::embedding_model::EmbeddingModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSet {
    pub embedding_set_id: Uuid,
    pub chunk_set_id: Uuid,
    pub embedding_model_id: Uuid,
    /// Snapshot of the embedding model config at the time of embedding.
    pub embedding_model_snapshot: EmbeddingModel,
    pub dimensions: u32,
    pub created_at: String,
}

/// A single chunk's embedding vector within an EmbeddingSet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkEmbedding {
    pub chunk_id: Uuid,
    pub embedding_set_id: Uuid,
    pub vector: Vec<f32>,
}
