use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::configuration::events::ConfigurationEvent;

#[async_trait]
pub trait ConfigurationEventStore: Send + Sync {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<ConfigurationEvent>, AppError>;

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[ConfigurationEvent],
    ) -> Result<(), AppError>;
}
