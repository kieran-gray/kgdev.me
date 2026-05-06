use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::{EvaluationRunResult, EvaluationRunSummary};

#[async_trait]
pub trait EvaluationResultStore: Send + Sync {
    async fn load(
        &self,
        slug: &str,
        version: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError>;
    async fn list(&self, slug: &str, version: &str) -> Result<Vec<EvaluationRunSummary>, AppError>;
    async fn load_run(
        &self,
        slug: &str,
        version: &str,
        run_id: &str,
    ) -> Result<Option<EvaluationRunResult>, AppError>;
    async fn store(&self, result: &EvaluationRunResult) -> Result<(), AppError>;
}
