use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::indexing::events::IndexingEvent;

#[async_trait]
pub trait IndexingEventStore: Send + Sync {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<IndexingEvent>, AppError>;

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[IndexingEvent],
    ) -> Result<(), AppError>;
}
