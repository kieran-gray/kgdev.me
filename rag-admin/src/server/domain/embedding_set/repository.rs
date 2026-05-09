use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::entity::{ChunkEmbedding, EmbeddingSet};

#[derive(Debug, Error)]
pub enum EmbeddingSetRepositoryError {
    #[error("embedding set repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait EmbeddingSetRepository: Send + Sync {
    async fn save(
        &self,
        embedding_set: EmbeddingSet,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), EmbeddingSetRepositoryError>;

    async fn load(
        &self,
        embedding_set_id: Uuid,
    ) -> Result<Option<EmbeddingSet>, EmbeddingSetRepositoryError>;

    /// Dedup lookup: returns an existing EmbeddingSet if the same chunks have already
    /// been embedded with the same model. Used to skip re-embedding on re-index.
    async fn find_by(
        &self,
        chunk_set_id: Uuid,
        embedding_model_id: Uuid,
    ) -> Result<Option<EmbeddingSet>, EmbeddingSetRepositoryError>;

    async fn load_embeddings(
        &self,
        embedding_set_id: Uuid,
    ) -> Result<Vec<ChunkEmbedding>, EmbeddingSetRepositoryError>;
}
