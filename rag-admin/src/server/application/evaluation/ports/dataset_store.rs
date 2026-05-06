use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::{EvaluationDataset, EvaluationDatasetStatus};

#[async_trait]
pub trait EvaluationDatasetStore: Send + Sync {
    async fn load(&self, slug: &str, version: &str) -> Result<EvaluationDataset, AppError>;
    async fn status(&self, slug: &str, version: &str) -> Result<EvaluationDatasetStatus, AppError>;
    async fn store(&self, dataset: &EvaluationDataset) -> Result<(), AppError>;
}
