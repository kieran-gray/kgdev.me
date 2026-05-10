use async_trait::async_trait;
use uuid::Uuid;

use crate::server::application::AppError;
use crate::server::domain::evaluation::dataset::events::EvaluationDatasetEvent;
use crate::server::domain::evaluation::run::events::EvaluationRunEvent;

#[async_trait]
pub trait EvaluationDatasetEventStore: Send + Sync {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<EvaluationDatasetEvent>, AppError>;

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[EvaluationDatasetEvent],
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait EvaluationRunEventStore: Send + Sync {
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<EvaluationRunEvent>, AppError>;

    async fn append(
        &self,
        aggregate_id: Uuid,
        expected_version: usize,
        events: &[EvaluationRunEvent],
    ) -> Result<(), AppError>;
}
