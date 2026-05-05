use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::EvaluationRunResult;

#[async_trait]
pub trait EvaluationResultStore: Send + Sync {
    async fn load(
        &self,
        slug: &str,
        version: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError>;
    async fn store(&self, result: &EvaluationRunResult) -> Result<(), AppError>;
}
