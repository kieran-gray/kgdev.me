use std::sync::Arc;

use tracing::info;

use crate::server::application::AppError;
use crate::server::domain::source_document::{
    aggregate::SourceDocument, commands::SourceDocumentCommand, events::SourceDocumentEvent,
    projector::SourceDocumentProjector, repository::SourceDocumentRepository,
};
use crate::server::domain::Aggregate;

use super::ports::SourceDocumentEventStore;

pub struct SourceDocumentCommandHandler {
    event_store: Arc<dyn SourceDocumentEventStore>,
    repository: Arc<dyn SourceDocumentRepository>,
}

impl SourceDocumentCommandHandler {
    pub fn new(
        event_store: Arc<dyn SourceDocumentEventStore>,
        repository: Arc<dyn SourceDocumentRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_store,
            repository,
        })
    }

    pub async fn handle(&self, command: SourceDocumentCommand) -> Result<(), AppError> {
        let aggregate_id = command.document_id();
        info!(%aggregate_id, "processing source document command");

        let stored_events = self.event_store.load(aggregate_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(SourceDocument::from_events(&stored_events).ok_or_else(|| {
                AppError::Internal(
                    "source document event stream is invalid: missing or duplicate create event"
                        .into(),
                )
            })?)
        };

        let new_events = SourceDocument::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(aggregate_id, previous_version, &new_events)
                .await?;
        }

        let all_events: Vec<SourceDocumentEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        if let Some(read_model) = SourceDocumentProjector::project(&all_events) {
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
    use crate::server::domain::{
        shared::Timestamp,
        source_document::{
            commands::{CreateDocument, NewVersion},
            document_type::DocumentType,
            events::SourceDocumentEvent,
            read_model::SourceDocumentReadModel,
            repository::SourceDocumentRepositoryError,
            source_ref::SourceRef,
            version::{BlogPostMetadata, ContentHash, DocumentMetadata},
        },
    };

    #[derive(Default)]
    struct MockEventStore {
        events: Mutex<Vec<SourceDocumentEvent>>,
        append_calls: Mutex<Vec<(Uuid, usize, Vec<SourceDocumentEvent>)>>,
    }

    #[async_trait]
    impl SourceDocumentEventStore for MockEventStore {
        async fn load(&self, _id: Uuid) -> Result<Vec<SourceDocumentEvent>, AppError> {
            Ok(self.events.lock().unwrap().clone())
        }

        async fn append(
            &self,
            id: Uuid,
            expected_version: usize,
            events: &[SourceDocumentEvent],
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
        saved: Mutex<Vec<SourceDocumentReadModel>>,
    }

    #[async_trait]
    impl SourceDocumentRepository for MockRepository {
        async fn load(
            &self,
            _id: Uuid,
        ) -> Result<Option<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
            Ok(self.saved.lock().unwrap().last().cloned())
        }

        async fn save(
            &self,
            read_model: SourceDocumentReadModel,
        ) -> Result<(), SourceDocumentRepositoryError> {
            self.saved.lock().unwrap().push(read_model);
            Ok(())
        }

        async fn list(
            &self,
        ) -> Result<Vec<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
            Ok(self.saved.lock().unwrap().clone())
        }

        async fn find_by_source_ref(
            &self,
            _source_ref: &crate::server::domain::source_document::source_ref::SourceRef,
        ) -> Result<Option<SourceDocumentReadModel>, SourceDocumentRepositoryError> {
            Ok(None)
        }
    }

    fn now() -> Timestamp {
        "2024-01-01T00:00:00Z".into()
    }

    fn make_create_command(document_id: Uuid) -> SourceDocumentCommand {
        SourceDocumentCommand::CreateDocument(CreateDocument {
            document_id,
            document_type: DocumentType::BlogPost,
            source_ref: SourceRef::UpstreamSlug {
                slug: "my-post".to_string(),
            },
            initial_version: NewVersion {
                content_hash: ContentHash::new("abc123".to_string()),
                metadata: DocumentMetadata::BlogPost(BlogPostMetadata {
                    title: "My Post".to_string(),
                    published_at: "2024-01-01".to_string(),
                }),
            },
            occurred_at: now(),
        })
    }

    #[tokio::test]
    async fn create_appends_events_and_saves_read_model() {
        let store = Arc::new(MockEventStore::default());
        let repo = Arc::new(MockRepository::default());
        let handler = SourceDocumentCommandHandler::new(store.clone(), repo.clone());

        let document_id = Uuid::new_v4();
        handler
            .handle(make_create_command(document_id))
            .await
            .unwrap();

        let calls = store.append_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, document_id);
        assert_eq!(calls[0].1, 0);
        assert_eq!(calls[0].2.len(), 2);

        let saved = repo.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].document_id, document_id);
        assert_eq!(saved[0].latest_version_number, 1);
    }

    #[tokio::test]
    async fn existing_stream_uses_loaded_version_for_append() {
        let document_id = Uuid::new_v4();
        let initial_events = {
            let mut events = vec![];
            let cmd = make_create_command(document_id);
            let new = SourceDocument::handle_command(None, cmd).unwrap();
            events.extend(new);
            events
        };

        let store = Arc::new(MockEventStore {
            events: Mutex::new(initial_events),
            append_calls: Mutex::new(vec![]),
        });
        let repo = Arc::new(MockRepository::default());
        let handler = SourceDocumentCommandHandler::new(store.clone(), repo.clone());

        use crate::server::domain::source_document::commands::{AddVersion, NewVersion};
        handler
            .handle(SourceDocumentCommand::AddVersion(AddVersion {
                document_id,
                version: NewVersion {
                    content_hash: ContentHash::new("def456".to_string()),
                    metadata: DocumentMetadata::BlogPost(BlogPostMetadata {
                        title: "Updated".to_string(),
                        published_at: "2024-02-01".to_string(),
                    }),
                },
                occurred_at: now(),
            }))
            .await
            .unwrap();

        let calls = store.append_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, 2); // after DocumentCreated + VersionAdded
        assert_eq!(calls[0].2.len(), 1);

        let saved = repo.saved.lock().unwrap();
        assert_eq!(saved[0].latest_version_number, 2);
    }

    #[tokio::test]
    async fn idempotent_add_version_does_not_append() {
        let document_id = Uuid::new_v4();
        let initial_events = {
            let cmd = make_create_command(document_id);
            SourceDocument::handle_command(None, cmd).unwrap()
        };

        let store = Arc::new(MockEventStore {
            events: Mutex::new(initial_events),
            append_calls: Mutex::new(vec![]),
        });
        let repo = Arc::new(MockRepository::default());
        let handler = SourceDocumentCommandHandler::new(store.clone(), repo.clone());

        use crate::server::domain::source_document::commands::{AddVersion, NewVersion};
        handler
            .handle(SourceDocumentCommand::AddVersion(AddVersion {
                document_id,
                version: NewVersion {
                    content_hash: ContentHash::new("abc123".to_string()), // same as initial
                    metadata: DocumentMetadata::BlogPost(BlogPostMetadata {
                        title: "My Post".to_string(),
                        published_at: "2024-01-01".to_string(),
                    }),
                },
                occurred_at: now(),
            }))
            .await
            .unwrap();

        let calls = store.append_calls.lock().unwrap();
        assert!(calls.is_empty(), "no events should be appended for a no-op");
    }
}
