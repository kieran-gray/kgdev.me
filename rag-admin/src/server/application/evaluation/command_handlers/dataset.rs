use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::server::application::evaluation::ports::EvaluationDatasetEventStore;
use crate::server::application::AppError;
use crate::server::domain::evaluation::dataset::{
    aggregate::EvaluationDataset, commands::EvaluationDatasetCommand,
    events::EvaluationDatasetEvent, projector::EvaluationDatasetProjector,
    repository::EvaluationDatasetRepository,
};
use crate::server::domain::Aggregate;

pub struct EvaluationDatasetCommandHandler {
    event_store: Arc<dyn EvaluationDatasetEventStore>,
    repository: Arc<dyn EvaluationDatasetRepository>,
}

impl EvaluationDatasetCommandHandler {
    pub fn new(
        event_store: Arc<dyn EvaluationDatasetEventStore>,
        repository: Arc<dyn EvaluationDatasetRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_store,
            repository,
        })
    }

    pub async fn handle(
        &self,
        dataset_id: Uuid,
        command: EvaluationDatasetCommand,
    ) -> Result<(), AppError> {
        info!(%dataset_id, "processing evaluation dataset command");

        let stored_events = self.event_store.load(dataset_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(
                EvaluationDataset::from_events(&stored_events).ok_or_else(|| {
                    AppError::Internal("evaluation dataset event stream is invalid".into())
                })?,
            )
        };

        let new_events = EvaluationDataset::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(dataset_id, previous_version, &new_events)
                .await?;
        }

        let all_events: Vec<EvaluationDatasetEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        if let Some(read_model) = EvaluationDatasetProjector::project(&all_events) {
            self.repository.save(read_model).await?;
        }

        Ok(())
    }

    /// Handle an `AcceptQuestion` command and also persist the question row.
    pub async fn handle_accept_question(
        &self,
        dataset_id: Uuid,
        command: EvaluationDatasetCommand,
        question: crate::server::domain::evaluation::question::EvaluationQuestion,
    ) -> Result<(), AppError> {
        self.handle(dataset_id, command).await?;
        self.repository.save_question(dataset_id, question).await?;
        Ok(())
    }
}
