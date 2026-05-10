use uuid::Uuid;

use crate::server::domain::shared::Timestamp;

use super::super::question::EvaluationReference;

pub struct RequestDatasetGeneration {
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
    pub occurred_at: Timestamp,
}

pub struct AcceptQuestion {
    pub sequence: u32,
    pub question: String,
    pub references: Vec<EvaluationReference>,
    pub embedding: Option<Vec<f32>>,
    pub occurred_at: Timestamp,
}

pub struct RejectQuestion {
    pub attempt: u32,
    pub reason: String,
    pub occurred_at: Timestamp,
}

pub struct CompleteDatasetGeneration {
    pub occurred_at: Timestamp,
}

pub struct FailDatasetGeneration {
    pub reason: String,
    pub occurred_at: Timestamp,
}

pub enum EvaluationDatasetCommand {
    RequestDatasetGeneration(RequestDatasetGeneration),
    AcceptQuestion(AcceptQuestion),
    RejectQuestion(RejectQuestion),
    CompleteDatasetGeneration(CompleteDatasetGeneration),
    FailDatasetGeneration(FailDatasetGeneration),
}
