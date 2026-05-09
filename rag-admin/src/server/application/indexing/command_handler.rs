use std::sync::Arc;

use tracing::info;

use crate::server::application::AppError;
use crate::server::domain::indexing::{
    aggregate::Indexing, commands::IndexingCommand, events::IndexingEvent,
    projector::IndexingProjector, repository::IndexingRepository,
};
use crate::server::domain::Aggregate;

use super::ports::IndexingEventStore;

pub struct IndexingCommandHandler {
    event_store: Arc<dyn IndexingEventStore>,
    repository: Arc<dyn IndexingRepository>,
}

impl IndexingCommandHandler {
    pub fn new(
        event_store: Arc<dyn IndexingEventStore>,
        repository: Arc<dyn IndexingRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_store,
            repository,
        })
    }

    pub async fn handle(&self, command: IndexingCommand) -> Result<(), AppError> {
        let aggregate_id = Self::aggregate_id_for(&command);
        info!(%aggregate_id, "processing indexing command");

        let stored_events = self.event_store.load(aggregate_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(Indexing::from_events(&stored_events).ok_or_else(|| {
                AppError::Internal(
                    "indexing event stream is invalid: missing or duplicate create event".into(),
                )
            })?)
        };

        let new_events = Indexing::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(aggregate_id, previous_version, &new_events)
                .await?;
        }

        let all_events: Vec<IndexingEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        if let Some(read_model) = IndexingProjector::project(&all_events) {
            self.repository.save(read_model).await?;
        }

        Ok(())
    }

    fn aggregate_id_for(command: &IndexingCommand) -> uuid::Uuid {
        match command {
            IndexingCommand::RequestIngest(cmd) => {
                Indexing::compute_id(cmd.document_id, cmd.pipeline_configuration_id)
            }
            IndexingCommand::CompleteChunking(_)
            | IndexingCommand::CompleteEmbedding(_)
            | IndexingCommand::CompleteIndexing(_)
            | IndexingCommand::FailIngestion(_)
            | IndexingCommand::RetryIngestion(_)
            | IndexingCommand::RemoveIndexing(_) => {
                panic!("stage completion commands must be dispatched with a known aggregate_id; use handle_for(aggregate_id, command)")
            }
        }
    }

    /// Handle a stage-completion command for a known aggregate_id (returned from RequestIngest).
    pub async fn handle_for(
        &self,
        aggregate_id: uuid::Uuid,
        command: IndexingCommand,
    ) -> Result<(), AppError> {
        info!(%aggregate_id, "processing indexing stage command");

        let stored_events = self.event_store.load(aggregate_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(
                Indexing::from_events(&stored_events)
                    .ok_or_else(|| AppError::Internal("indexing event stream is invalid".into()))?,
            )
        };

        let new_events = Indexing::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(aggregate_id, previous_version, &new_events)
                .await?;
        }

        let all_events: Vec<IndexingEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        if let Some(read_model) = IndexingProjector::project(&all_events) {
            self.repository.save(read_model).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use crate::server::domain::indexing::{
        commands::{CompleteChunking, RequestIngest},
        events::IndexingEvent,
        read_model::IndexingReadModel,
        repository::IndexingRepositoryError,
        status::IndexingStatus,
    };
    use crate::shared::ChunkingConfig;
    use crate::shared::SectionChunkingConfig;

    #[derive(Default)]
    struct MockEventStore {
        events: Mutex<Vec<IndexingEvent>>,
        append_calls: Mutex<Vec<(Uuid, usize, Vec<IndexingEvent>)>>,
    }

    #[async_trait]
    impl IndexingEventStore for MockEventStore {
        async fn load(&self, _id: Uuid) -> Result<Vec<IndexingEvent>, AppError> {
            Ok(self.events.lock().unwrap().clone())
        }

        async fn append(
            &self,
            id: Uuid,
            expected_version: usize,
            events: &[IndexingEvent],
        ) -> Result<(), AppError> {
            self.append_calls
                .lock()
                .unwrap()
                .push((id, expected_version, events.to_vec()));
            self.events.lock().unwrap().extend(events.iter().cloned());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockRepository {
        saved: Mutex<Vec<IndexingReadModel>>,
    }

    #[async_trait]
    impl IndexingRepository for MockRepository {
        async fn load(
            &self,
            _id: Uuid,
        ) -> Result<Option<IndexingReadModel>, IndexingRepositoryError> {
            Ok(self.saved.lock().unwrap().last().cloned())
        }

        async fn save(&self, read_model: IndexingReadModel) -> Result<(), IndexingRepositoryError> {
            self.saved.lock().unwrap().push(read_model);
            Ok(())
        }

        async fn list_for_document(
            &self,
            _document_id: Uuid,
        ) -> Result<Vec<IndexingReadModel>, IndexingRepositoryError> {
            Ok(self.saved.lock().unwrap().clone())
        }
    }

    fn make_request(doc_id: Uuid, pc_id: Uuid) -> IndexingCommand {
        IndexingCommand::RequestIngest(RequestIngest {
            document_id: doc_id,
            pipeline_configuration_id: pc_id,
            document_version: 1,
            chunking_config: ChunkingConfig::Section(SectionChunkingConfig {
                max_section_tokens: 512,
            }),
            request_id: Uuid::new_v4(),
            occurred_at: "2024-01-01T00:00:00Z".to_string(),
        })
    }

    #[tokio::test]
    async fn request_appends_event_and_saves_read_model() {
        let store = Arc::new(MockEventStore::default());
        let repo = Arc::new(MockRepository::default());
        let handler = IndexingCommandHandler::new(store.clone(), repo.clone());

        let doc_id = Uuid::new_v4();
        let pc_id = Uuid::new_v4();
        handler.handle(make_request(doc_id, pc_id)).await.unwrap();

        let calls = store.append_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, 0);

        let saved = repo.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].status, IndexingStatus::Pending);
        assert_eq!(saved[0].document_id, doc_id);
    }

    #[tokio::test]
    async fn handle_for_stage_command_uses_correct_stream() {
        let doc_id = Uuid::new_v4();
        let pc_id = Uuid::new_v4();
        let aggregate_id = Indexing::compute_id(doc_id, pc_id);

        let initial_events = {
            Indexing::handle_command(
                None,
                IndexingCommand::RequestIngest(RequestIngest {
                    document_id: doc_id,
                    pipeline_configuration_id: pc_id,
                    document_version: 1,
                    chunking_config: ChunkingConfig::Section(SectionChunkingConfig {
                        max_section_tokens: 512,
                    }),
                    request_id: Uuid::new_v4(),
                    occurred_at: "2024-01-01T00:00:00Z".to_string(),
                }),
            )
            .unwrap()
        };

        let store = Arc::new(MockEventStore {
            events: Mutex::new(initial_events),
            append_calls: Mutex::new(vec![]),
        });
        let repo = Arc::new(MockRepository::default());
        let handler = IndexingCommandHandler::new(store.clone(), repo.clone());

        handler
            .handle_for(
                aggregate_id,
                IndexingCommand::CompleteChunking(CompleteChunking {
                    chunk_set_id: Uuid::new_v4(),
                    chunk_count: 20,
                    occurred_at: "2024-01-01T00:01:00Z".to_string(),
                }),
            )
            .await
            .unwrap();

        let calls = store.append_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, 1); // after IngestRequested

        let saved = repo.saved.lock().unwrap();
        assert_eq!(saved[0].status, IndexingStatus::Chunking);
    }
}
