use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::source_document::events::SourceDocumentEvent;

#[async_trait]
pub trait SourceDocumentEventStore: Send + Sync {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<SourceDocumentEvent>, AppError>;

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[SourceDocumentEvent],
    ) -> Result<(), AppError>;
}
