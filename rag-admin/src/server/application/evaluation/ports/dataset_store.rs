use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::{EvaluationDatasetDto, EvaluationDatasetStatus};

#[async_trait]
pub trait EvaluationDatasetStore: Send + Sync {
    async fn load(&self, slug: &str, version: &str) -> Result<EvaluationDatasetDto, AppError>;
    async fn status(&self, slug: &str, version: &str) -> Result<EvaluationDatasetStatus, AppError>;
    async fn store(&self, dataset: &EvaluationDatasetDto) -> Result<(), AppError>;
}
