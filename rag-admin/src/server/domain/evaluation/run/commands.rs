use uuid::Uuid;

use crate::server::domain::shared::Timestamp;
use crate::shared::{
    ChunkingVariant, EvaluationAutotuneRequest, EvaluationMetrics, EvaluationResultSplit,
    EvaluationRunOptions,
};

use super::{events::RetrievalTraceEntry, scoring_policy::ScoringPolicy};

pub struct RequestRun {
    pub run_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub variants: Vec<ChunkingVariant>,
    pub options: Vec<EvaluationRunOptions>,
    pub autotune_request: Option<EvaluationAutotuneRequest>,
    pub scoring_policy: ScoringPolicy,
    pub occurred_at: Timestamp,
}

pub struct MarkVariantPrepared {
    pub variant_label: String,
    pub chunk_set_id: Uuid,
    pub embedding_set_id: Uuid,
    pub occurred_at: Timestamp,
}

pub struct ScoreVariant {
    pub variant_label: String,
    pub split: EvaluationResultSplit,
    pub metrics: EvaluationMetrics,
    pub retrieval_traces: Vec<RetrievalTraceEntry>,
    pub selected: bool,
    pub occurred_at: Timestamp,
}

pub struct CompleteRun {
    pub occurred_at: Timestamp,
}

pub struct FailRun {
    pub reason: String,
    pub occurred_at: Timestamp,
}

pub enum EvaluationRunCommand {
    RequestRun(RequestRun),
    MarkVariantPrepared(MarkVariantPrepared),
    ScoreVariant(ScoreVariant),
    CompleteRun(CompleteRun),
    FailRun(FailRun),
}
