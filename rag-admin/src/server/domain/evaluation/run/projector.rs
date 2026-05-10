use crate::server::domain::Aggregate;

use super::{
    aggregate::EvaluationRun, events::EvaluationRunEvent, read_model::EvaluationRunReadModel,
};

pub struct EvaluationRunProjector;

impl EvaluationRunProjector {
    pub fn project(events: &[EvaluationRunEvent]) -> Option<EvaluationRunReadModel> {
        EvaluationRun::from_events(events).map(|r| EvaluationRunReadModel::from(&r))
    }
}
