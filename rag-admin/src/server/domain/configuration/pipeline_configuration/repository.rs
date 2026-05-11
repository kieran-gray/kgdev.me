use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::read_model::PipelineConfigurationReadModel;

#[derive(Debug, Error)]
pub enum PipelineConfigurationRepositoryError {
    #[error("pipeline configuration repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait PipelineConfigurationRepository: Send + Sync {
    async fn load_all(
        &self,
    ) -> Result<Vec<PipelineConfigurationReadModel>, PipelineConfigurationRepositoryError>;

    async fn save(
        &self,
        read_model: PipelineConfigurationReadModel,
    ) -> Result<(), PipelineConfigurationRepositoryError>;

    async fn delete(&self, id: Uuid) -> Result<(), PipelineConfigurationRepositoryError>;
}
