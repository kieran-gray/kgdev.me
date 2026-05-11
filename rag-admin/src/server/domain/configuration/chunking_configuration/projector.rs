use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::configuration::events::ConfigurationEvent;
use crate::server::event_sourcing::envelope::EventEnvelope;
use crate::server::event_sourcing::projector::Projector;

use super::read_model::ChunkingConfigurationReadModel;
use super::repository::ChunkingConfigurationRepository;

pub struct ChunkingConfigurationProjector {
    repository: Arc<dyn ChunkingConfigurationRepository>,
}

impl ChunkingConfigurationProjector {
    pub const NAME: &'static str = "chunking_configuration_projector";

    pub fn new(repository: Arc<dyn ChunkingConfigurationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl Projector<ConfigurationEvent> for ChunkingConfigurationProjector {
    fn name(&self) -> &str {
        Self::NAME
    }

    async fn project(
        &self,
        events: &[EventEnvelope<ConfigurationEvent>],
    ) -> Result<(), AppError> {
        for envelope in events {
            match &envelope.event {
                ConfigurationEvent::ChunkingConfigurationCreated(e) => {
                    self.repository
                        .save(ChunkingConfigurationReadModel {
                            chunking_configuration_id: e.chunking_configuration_id,
                            name: e.name.clone(),
                            config: e.config.clone(),
                        })
                        .await?;
                }
                ConfigurationEvent::ChunkingConfigurationUpdated(e) => {
                    self.repository
                        .save(ChunkingConfigurationReadModel {
                            chunking_configuration_id: e.chunking_configuration_id,
                            name: e.name.clone(),
                            config: e.config.clone(),
                        })
                        .await?;
                }
                ConfigurationEvent::ChunkingConfigurationDeleted(e) => {
                    self.repository
                        .delete(e.chunking_configuration_id)
                        .await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
