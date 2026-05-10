use crate::server::domain::Aggregate;

use super::{
    aggregate::EvaluationDataset, events::EvaluationDatasetEvent,
    read_model::EvaluationDatasetReadModel,
};

pub struct EvaluationDatasetProjector;

impl EvaluationDatasetProjector {
    pub fn project(events: &[EvaluationDatasetEvent]) -> Option<EvaluationDatasetReadModel> {
        EvaluationDataset::from_events(events).map(|d| EvaluationDatasetReadModel::from(&d))
    }
}
