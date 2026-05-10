use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::server::application::evaluation::ports::EvaluationRunEventStore;
use crate::server::application::AppError;
use crate::server::domain::evaluation::run::{
    aggregate::EvaluationRun, commands::EvaluationRunCommand, events::EvaluationRunEvent,
    projector::EvaluationRunProjector, repository::EvaluationRunRepository,
};
use crate::server::domain::Aggregate;

pub struct EvaluationRunCommandHandler {
    event_store: Arc<dyn EvaluationRunEventStore>,
    repository: Arc<dyn EvaluationRunRepository>,
}

impl EvaluationRunCommandHandler {
    pub fn new(
        event_store: Arc<dyn EvaluationRunEventStore>,
        repository: Arc<dyn EvaluationRunRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_store,
            repository,
        })
    }

    pub async fn handle(
        &self,
        run_id: Uuid,
        command: EvaluationRunCommand,
    ) -> Result<(), AppError> {
        info!(%run_id, "processing evaluation run command");

        let stored_events = self.event_store.load(run_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(EvaluationRun::from_events(&stored_events).ok_or_else(|| {
                AppError::Internal("evaluation run event stream is invalid".into())
            })?)
        };

        let new_events = EvaluationRun::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(run_id, previous_version, &new_events)
                .await?;
        }

        let all_events: Vec<EvaluationRunEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        if let Some(read_model) = EvaluationRunProjector::project(&all_events) {
            self.repository.save(read_model).await?;
        }

        Ok(())
    }

    /// Handle a `ScoreVariant` command and also persist the variant result row.
    pub async fn handle_score_variant(
        &self,
        run_id: Uuid,
        command: EvaluationRunCommand,
        variant_result: crate::server::domain::evaluation::run::read_model::EvaluationVariantResultDto,
    ) -> Result<(), AppError> {
        self.handle(run_id, command).await?;
        self.repository.save_variant_result(variant_result).await?;
        Ok(())
    }
}
