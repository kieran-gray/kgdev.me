use std::sync::Arc;

use tracing::info;

use crate::server::application::configuration::ports::ConfigurationEventStore;
use crate::server::application::AppError;
use crate::server::domain::configuration::chunking_configuration::{
    ChunkingConfigurationProjector, ChunkingConfigurationRepository,
};
use crate::server::domain::configuration::pipeline_configuration::{
    PipelineConfigurationProjector, PipelineConfigurationRepository,
};
use crate::server::domain::configuration::{
    aggregate::Configuration, commands::ConfigurationCommand, events::ConfigurationEvent,
    ConfigurationProjector, ConfigurationRepository,
};
use crate::server::domain::Aggregate;
use crate::shared::ConfigurationCommandDto;

pub struct ConfigurationCommandHandler {
    event_store: Arc<dyn ConfigurationEventStore>,
    configuration_repository: Arc<dyn ConfigurationRepository>,
    pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
    chunking_configuration_repository: Arc<dyn ChunkingConfigurationRepository>,
}

impl ConfigurationCommandHandler {
    pub fn new(
        event_store: Arc<dyn ConfigurationEventStore>,
        configuration_repository: Arc<dyn ConfigurationRepository>,
        pipeline_configuration_repository: Arc<dyn PipelineConfigurationRepository>,
        chunking_configuration_repository: Arc<dyn ChunkingConfigurationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_store,
            configuration_repository,
            pipeline_configuration_repository,
            chunking_configuration_repository,
        })
    }

    pub async fn handle(&self, command: ConfigurationCommand) -> Result<(), AppError> {
        info!("Processing configuration command");

        let aggregate_id = Configuration::singleton_id();
        let stored_events = self.event_store.load(aggregate_id).await?;
        let previous_version = stored_events.len();

        let state = if stored_events.is_empty() {
            None
        } else {
            Some(Configuration::from_events(&stored_events).ok_or_else(|| {
                AppError::Internal(
                    "configuration event stream is invalid: missing or duplicate create event"
                        .into(),
                )
            })?)
        };

        let new_events = Configuration::handle_command(state.as_ref(), command)?;

        if !new_events.is_empty() {
            self.event_store
                .append(aggregate_id, previous_version, &new_events)
                .await?;
        }

        // TODO: projectors should be incremental
        let all_events: Vec<ConfigurationEvent> = stored_events
            .iter()
            .chain(new_events.iter())
            .cloned()
            .collect();

        self.configuration_repository
            .save(ConfigurationProjector::project(&all_events))
            .await?;

        self.pipeline_configuration_repository
            .rebuild(&PipelineConfigurationProjector::from_events(&all_events))
            .await?;

        self.chunking_configuration_repository
            .rebuild(&ChunkingConfigurationProjector::from_events(&all_events))
            .await?;

        Ok(())
    }

    pub async fn handle_dto(&self, command: ConfigurationCommandDto) -> Result<(), AppError> {
        self.handle(command.into()).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use crate::server::domain::configuration::chunking_configuration::{
        ChunkingConfigurationReadModel, ChunkingConfigurationRepositoryError,
    };
    use crate::server::domain::configuration::embedding_model::commands::AddEmbeddingModel;
    use crate::server::domain::configuration::events::{
        ConfigurationCreated, ConfigurationEvent, EmbeddingModelAdded,
    };
    use crate::server::domain::configuration::kinds::AiProviderKind;
    use crate::server::domain::configuration::pipeline_configuration::{
        PipelineConfigurationReadModel, PipelineConfigurationRepositoryError,
    };
    use crate::server::domain::configuration::{
        read_model::ConfigurationReadModel,
        repository::{ConfigurationRepository, ConfigurationRepositoryError},
    };

    #[derive(Default)]
    struct MockConfigurationEventStore {
        events: Mutex<Vec<ConfigurationEvent>>,
        append_calls: Mutex<Vec<(Uuid, usize, Vec<ConfigurationEvent>)>>,
    }

    #[async_trait]
    impl ConfigurationEventStore for MockConfigurationEventStore {
        async fn load(&self, _aggregate_id: Uuid) -> Result<Vec<ConfigurationEvent>, AppError> {
            Ok(self.events.lock().unwrap().clone())
        }

        async fn append(
            &self,
            aggregate_id: Uuid,
            expected_version: usize,
            events: &[ConfigurationEvent],
        ) -> Result<(), AppError> {
            self.append_calls.lock().unwrap().push((
                aggregate_id,
                expected_version,
                events.to_vec(),
            ));
            self.events.lock().unwrap().extend(events.iter().cloned());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockConfigurationRepository {
        saved: Mutex<Vec<ConfigurationReadModel>>,
    }

    #[async_trait]
    impl ConfigurationRepository for MockConfigurationRepository {
        async fn load(&self) -> Result<ConfigurationReadModel, ConfigurationRepositoryError> {
            Ok(self
                .saved
                .lock()
                .unwrap()
                .last()
                .cloned()
                .unwrap_or_default())
        }

        async fn save(
            &self,
            read_model: ConfigurationReadModel,
        ) -> Result<(), ConfigurationRepositoryError> {
            self.saved.lock().unwrap().push(read_model);
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockPipelineConfigurationRepository {
        saved: Mutex<Vec<PipelineConfigurationReadModel>>,
        deleted: Mutex<Vec<Uuid>>,
    }

    #[async_trait]
    impl PipelineConfigurationRepository for MockPipelineConfigurationRepository {
        async fn load_all(
            &self,
        ) -> Result<Vec<PipelineConfigurationReadModel>, PipelineConfigurationRepositoryError>
        {
            Ok(self.saved.lock().unwrap().clone())
        }

        async fn save(
            &self,
            read_model: PipelineConfigurationReadModel,
        ) -> Result<(), PipelineConfigurationRepositoryError> {
            self.saved.lock().unwrap().push(read_model);
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), PipelineConfigurationRepositoryError> {
            self.deleted.lock().unwrap().push(id);
            Ok(())
        }

        async fn rebuild(
            &self,
            configurations: &[PipelineConfigurationReadModel],
        ) -> Result<(), PipelineConfigurationRepositoryError> {
            let mut guard = self.saved.lock().unwrap();
            *guard = configurations.to_vec();
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockChunkingConfigurationRepository {
        saved: Mutex<Vec<ChunkingConfigurationReadModel>>,
        deleted: Mutex<Vec<Uuid>>,
    }

    #[async_trait]
    impl ChunkingConfigurationRepository for MockChunkingConfigurationRepository {
        async fn load_all(
            &self,
        ) -> Result<Vec<ChunkingConfigurationReadModel>, ChunkingConfigurationRepositoryError>
        {
            Ok(self.saved.lock().unwrap().clone())
        }

        async fn save(
            &self,
            read_model: ChunkingConfigurationReadModel,
        ) -> Result<(), ChunkingConfigurationRepositoryError> {
            self.saved.lock().unwrap().push(read_model);
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), ChunkingConfigurationRepositoryError> {
            self.deleted.lock().unwrap().push(id);
            Ok(())
        }

        async fn rebuild(
            &self,
            configurations: &[ChunkingConfigurationReadModel],
        ) -> Result<(), ChunkingConfigurationRepositoryError> {
            let mut guard = self.saved.lock().unwrap();
            *guard = configurations.to_vec();
            Ok(())
        }
    }

    #[tokio::test]
    async fn first_command_creates_and_persists_configuration_stream() {
        let store = Arc::new(MockConfigurationEventStore::default());
        let config_repo = Arc::new(MockConfigurationRepository::default());
        let pc_repo = Arc::new(MockPipelineConfigurationRepository::default());
        let cc_repo = Arc::new(MockChunkingConfigurationRepository::default());
        let handler = ConfigurationCommandHandler::new(
            store.clone(),
            config_repo.clone(),
            pc_repo.clone(),
            cc_repo.clone(),
        );

        handler
            .handle(ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-base-en-v1.5".into(),
                dimensions: 768,
            }))
            .await
            .unwrap();

        let append_calls = store.append_calls.lock().unwrap();
        assert_eq!(append_calls.len(), 1);
        assert_eq!(append_calls[0].0, Configuration::singleton_id());
        assert_eq!(append_calls[0].1, 0);
        assert_eq!(append_calls[0].2.len(), 2);
        assert!(matches!(
            &append_calls[0].2[0],
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated { .. })
        ));
        assert!(matches!(
            &append_calls[0].2[1],
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded { .. })
        ));

        let saved = config_repo.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].embedding_models.len(), 1);
        assert_eq!(saved[0].embedding_models[0].model, "@cf/baai/bge-base-en-v1.5");
    }

    #[tokio::test]
    async fn existing_stream_uses_loaded_version_for_append() {
        let model_id = Uuid::new_v4();
        let store = Arc::new(MockConfigurationEventStore {
            events: Mutex::new(vec![
                ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                    configuration_id: Configuration::singleton_id(),
                }),
                ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                    model_id,
                    kind: AiProviderKind::Cloudflare,
                    model: "@cf/baai/bge-base-en-v1.5".into(),
                    dimensions: 768,
                }),
            ]),
            append_calls: Mutex::new(Vec::new()),
        });
        let config_repo = Arc::new(MockConfigurationRepository::default());
        let pc_repo = Arc::new(MockPipelineConfigurationRepository::default());
        let cc_repo = Arc::new(MockChunkingConfigurationRepository::default());
        let handler = ConfigurationCommandHandler::new(
            store.clone(),
            config_repo.clone(),
            pc_repo.clone(),
            cc_repo.clone(),
        );

        handler
            .handle(ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                kind: AiProviderKind::Ollama,
                model: "qwen3-embedding:0.6b".into(),
                dimensions: 1024,
            }))
            .await
            .unwrap();

        let append_calls = store.append_calls.lock().unwrap();
        assert_eq!(append_calls.len(), 1);
        assert_eq!(append_calls[0].1, 2);
        assert_eq!(append_calls[0].2.len(), 1);

        let saved = config_repo.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].embedding_models.len(), 2);
    }
}
