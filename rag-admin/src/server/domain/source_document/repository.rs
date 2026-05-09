use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::read_model::SourceDocumentReadModel;

#[derive(Debug, Error)]
pub enum SourceDocumentRepositoryError {
    #[error("source document repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait SourceDocumentRepository: Send + Sync {
    async fn load(
        &self,
        document_id: Uuid,
    ) -> Result<Option<SourceDocumentReadModel>, SourceDocumentRepositoryError>;

    async fn save(
        &self,
        read_model: SourceDocumentReadModel,
    ) -> Result<(), SourceDocumentRepositoryError>;

    async fn list(&self) -> Result<Vec<SourceDocumentReadModel>, SourceDocumentRepositoryError>;
}
