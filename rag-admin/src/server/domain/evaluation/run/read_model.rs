use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::{EvaluationMetrics, EvaluationResultSplit};

use super::{
    aggregate::{EvaluationRun, EvaluationRunStatus},
    events::RetrievalTraceEntry,
    scoring_policy::ScoringPolicy,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationVariantResultDto {
    pub run_id: Uuid,
    pub variant_label: String,
    pub split: EvaluationResultSplit,
    pub recall_mean: f32,
    pub recall_std: f32,
    pub precision_mean: f32,
    pub precision_std: f32,
    pub iou_mean: f32,
    pub iou_std: f32,
    pub precision_omega_mean: f32,
    pub precision_omega_std: f32,
    pub chunk_set_id: Uuid,
    pub embedding_set_id: Uuid,
    pub selected: bool,
    pub retrieval_traces: Vec<RetrievalTraceEntry>,
}

impl EvaluationVariantResultDto {
    pub fn metrics(&self) -> EvaluationMetrics {
        EvaluationMetrics {
            recall_mean: self.recall_mean,
            recall_std: self.recall_std,
            precision_mean: self.precision_mean,
            precision_std: self.precision_std,
            iou_mean: self.iou_mean,
            iou_std: self.iou_std,
            precision_omega_mean: self.precision_omega_mean,
            precision_omega_std: self.precision_omega_std,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRunReadModel {
    pub run_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_configuration_id: Uuid,
    pub document_id: Uuid,
    pub document_version: u32,
    pub status: EvaluationRunStatus,
    pub variants_count: u32,
    pub variants_prepared: u32,
    pub variants_scored: u32,
    pub failure_reason: Option<String>,
    pub scoring_recall_weight: f32,
    pub scoring_iou_weight: f32,
    pub scoring_precision_weight: f32,
    pub scoring_precision_omega_weight: f32,
    pub created_at: String,
    pub variant_results: Vec<EvaluationVariantResultDto>,
}

impl From<&EvaluationRun> for EvaluationRunReadModel {
    fn from(run: &EvaluationRun) -> Self {
        let failure_reason = match &run.status {
            EvaluationRunStatus::Failed { reason } => Some(reason.clone()),
            _ => None,
        };
        Self {
            run_id: run.run_id,
            dataset_id: run.dataset_id,
            pipeline_configuration_id: run.pipeline_configuration_id,
            document_id: run.document_id,
            document_version: run.document_version,
            status: run.status.clone(),
            variants_count: run.variants.len() as u32,
            variants_prepared: run.prepared_variants.len() as u32,
            variants_scored: run.scored_variants.len() as u32,
            failure_reason,
            scoring_recall_weight: run.scoring_policy.weights.recall,
            scoring_iou_weight: run.scoring_policy.weights.iou,
            scoring_precision_weight: run.scoring_policy.weights.precision,
            scoring_precision_omega_weight: run.scoring_policy.weights.precision_omega,
            created_at: run.created_at.to_string(),
            variant_results: Vec::new(),
        }
    }
}

impl EvaluationRunReadModel {
    pub fn scoring_policy(&self) -> ScoringPolicy {
        use super::scoring_policy::ScoringWeights;
        ScoringPolicy {
            weights: ScoringWeights {
                recall: self.scoring_recall_weight,
                iou: self.scoring_iou_weight,
                precision: self.scoring_precision_weight,
                precision_omega: self.scoring_precision_omega_weight,
            },
        }
    }
}

impl From<EvaluationVariantResultDto> for crate::shared::EvaluationVariantResult {
    fn from(v: EvaluationVariantResultDto) -> Self {
        Self {
            variant: crate::shared::ChunkingVariant {
                label: v.variant_label,
                config: crate::shared::ChunkingConfig::default(), // FIXME: where to get this?
            },
            options: crate::shared::EvaluationRunOptions::default(), // FIXME
            split: v.split,
            selected: v.selected,
            metrics: crate::shared::EvaluationMetrics {
                recall_mean: v.recall_mean,
                recall_std: v.recall_std,
                precision_mean: v.precision_mean,
                precision_std: v.precision_std,
                iou_mean: v.iou_mean,
                iou_std: v.iou_std,
                precision_omega_mean: v.precision_omega_mean,
                precision_omega_std: v.precision_omega_std,
            },
            chunk_count: 0,
            average_chunk_tokens: 0,
            average_retrieved_tokens: 0,
            question_results: Vec::new(),
        }
    }
}
