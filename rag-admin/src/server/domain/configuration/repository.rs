use async_trait::async_trait;
use thiserror::Error;

use super::read_model::ConfigurationReadModel;

#[derive(Debug, Error)]
pub enum ConfigurationRepositoryError {
    #[error("configuration repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ConfigurationRepository: Send + Sync {
    async fn load(&self) -> Result<ConfigurationReadModel, ConfigurationRepositoryError>;

    async fn save(
        &self,
        read_model: ConfigurationReadModel,
    ) -> Result<(), ConfigurationRepositoryError>;
}
