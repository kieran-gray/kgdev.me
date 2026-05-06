use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::VectorRecord;

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, index: &str, records: &[VectorRecord]) -> Result<(), AppError>;
    async fn delete_ids(&self, index: &str, ids: &[String]) -> Result<(), AppError>;
}
