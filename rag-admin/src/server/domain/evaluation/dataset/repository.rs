use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::server::domain::evaluation::question::EvaluationQuestion;

use super::read_model::EvaluationDatasetReadModel;

#[derive(Debug, Error)]
pub enum EvaluationDatasetRepositoryError {
    #[error("evaluation dataset repository error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait EvaluationDatasetRepository: Send + Sync {
    async fn load(
        &self,
        dataset_id: Uuid,
    ) -> Result<Option<EvaluationDatasetReadModel>, EvaluationDatasetRepositoryError>;

    async fn save(
        &self,
        read_model: EvaluationDatasetReadModel,
    ) -> Result<(), EvaluationDatasetRepositoryError>;

    async fn save_question(
        &self,
        dataset_id: Uuid,
        question: EvaluationQuestion,
    ) -> Result<(), EvaluationDatasetRepositoryError>;

    async fn list_for_document(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<EvaluationDatasetReadModel>, EvaluationDatasetRepositoryError>;

    async fn load_questions(
        &self,
        dataset_id: Uuid,
    ) -> Result<Vec<EvaluationQuestion>, EvaluationDatasetRepositoryError>;
}
