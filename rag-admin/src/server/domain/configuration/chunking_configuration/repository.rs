use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::read_model::ChunkingConfigurationReadModel;

#[derive(Debug, Error)]
pub enum ChunkingConfigurationRepositoryError {
    #[error("chunking configuration repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ChunkingConfigurationRepository: Send + Sync {
    async fn load_all(
        &self,
    ) -> Result<Vec<ChunkingConfigurationReadModel>, ChunkingConfigurationRepositoryError>;

    async fn save(
        &self,
        read_model: ChunkingConfigurationReadModel,
    ) -> Result<(), ChunkingConfigurationRepositoryError>;

    async fn delete(&self, id: Uuid) -> Result<(), ChunkingConfigurationRepositoryError>;
}
