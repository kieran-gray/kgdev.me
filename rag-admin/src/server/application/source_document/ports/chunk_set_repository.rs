use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::chunk_set::entity::{Chunk, ChunkSet};

#[async_trait]
pub trait ChunkSetRepository: Send + Sync {
    async fn save(&self, chunk_set: ChunkSet, chunks: Vec<Chunk>) -> Result<(), AppError>;

    async fn load(&self, chunk_set_id: Uuid) -> Result<Option<ChunkSet>, AppError>;

    async fn load_chunks(&self, chunk_set_id: Uuid) -> Result<Vec<Chunk>, AppError>;

    async fn list_for_document(&self, document_id: Uuid) -> Result<Vec<ChunkSet>, AppError>;
}
