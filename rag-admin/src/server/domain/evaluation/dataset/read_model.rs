use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::shared::Timestamp;

use super::aggregate::DatasetGenerationStatus;

/// Read model row for the `evaluation_datasets` table.
///
/// Built incrementally by `EvaluationDatasetProjector` from events; never
/// derived from the write-side aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationDatasetReadModel {
    pub dataset_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub content_hash: String,
    pub label: String,
    pub target_question_count: u32,
    pub generation_model_id: Uuid,
    pub generation_model: String,
    pub excerpt_similarity_threshold_milli: u32,
    pub duplicate_similarity_threshold_milli: u32,
    pub embedding_model_id: Uuid,
    pub status: DatasetGenerationStatus,
    pub question_count: u32,
    pub rejection_count: u32,
    pub failure_reason: Option<String>,
    pub created_at: Timestamp,
}

/// The subset of dataset fields written when a `DatasetGenerationRequested`
/// event is first projected. `question_count` and `rejection_count` start at
/// zero; `status` starts as Generating.
#[derive(Debug, Clone)]
pub struct NewDatasetSummary {
    pub dataset_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub content_hash: String,
    pub label: String,
    pub target_question_count: u32,
    pub generation_model_id: Uuid,
    pub generation_model: String,
    pub excerpt_similarity_threshold_milli: u32,
    pub duplicate_similarity_threshold_milli: u32,
    pub embedding_model_id: Uuid,
    pub created_at: Timestamp,
}
