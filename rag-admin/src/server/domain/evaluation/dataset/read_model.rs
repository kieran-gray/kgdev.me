use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::aggregate::{DatasetGenerationStatus, EvaluationDataset};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationDatasetReadModel {
    pub dataset_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub content_hash: String,
    pub label: String,
    pub target_question_count: u32,
    pub generation_model: String,
    pub generation_backend: String,
    pub excerpt_similarity_threshold_milli: u32,
    pub duplicate_similarity_threshold_milli: u32,
    pub embedding_model_id: Uuid,
    pub status: DatasetGenerationStatus,
    pub question_count: u32,
    pub rejection_count: u32,
    pub failure_reason: Option<String>,
    pub created_at: String,
}

impl From<&EvaluationDataset> for EvaluationDatasetReadModel {
    fn from(dataset: &EvaluationDataset) -> Self {
        let failure_reason = match &dataset.status {
            DatasetGenerationStatus::Failed { reason } => Some(reason.clone()),
            _ => None,
        };
        Self {
            dataset_id: dataset.dataset_id,
            document_id: dataset.document_id,
            document_version: dataset.document_version,
            content_hash: dataset.content_hash.clone(),
            label: dataset.label.clone(),
            target_question_count: dataset.target_question_count,
            generation_model: dataset.generation_model.clone(),
            generation_backend: dataset.generation_backend.clone(),
            excerpt_similarity_threshold_milli: dataset.excerpt_similarity_threshold_milli,
            duplicate_similarity_threshold_milli: dataset.duplicate_similarity_threshold_milli,
            embedding_model_id: dataset.embedding_model_id,
            status: dataset.status.clone(),
            question_count: dataset.questions.len() as u32,
            rejection_count: dataset.rejection_count,
            failure_reason,
            created_at: dataset.created_at.to_string(),
        }
    }
}
