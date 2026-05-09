use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::embedding_set::entity::{ChunkEmbedding, EmbeddingSet};

#[async_trait]
pub trait EmbeddingSetRepository: Send + Sync {
    async fn save(
        &self,
        embedding_set: EmbeddingSet,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), AppError>;

    async fn load(&self, embedding_set_id: Uuid) -> Result<Option<EmbeddingSet>, AppError>;

    /// Returns an existing EmbeddingSet if the same chunks have been embedded with
    /// the same model. Used to skip re-embedding on re-index.
    async fn find_by(
        &self,
        chunk_set_id: Uuid,
        embedding_model_id: Uuid,
    ) -> Result<Option<EmbeddingSet>, AppError>;

    async fn load_embeddings(
        &self,
        embedding_set_id: Uuid,
    ) -> Result<Vec<ChunkEmbedding>, AppError>;
}
