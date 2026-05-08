use async_trait::async_trait;
use thiserror::Error;

use crate::server::domain::pipeline_configuration::PipelineConfiguration;

#[derive(Debug, Error)]
pub enum PipelineConfigurationRepositoryError {
    #[error("pipeline configuration repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait PipelineConfigurationRepository: Send + Sync {
    async fn load(&self) -> Result<PipelineConfiguration, PipelineConfigurationRepositoryError>;

    async fn save(
        &self,
        pipeline_configuration: PipelineConfiguration,
    ) -> Result<(), PipelineConfigurationRepositoryError>;
}
