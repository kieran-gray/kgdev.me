use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::configuration::events::ConfigurationEvent;
use crate::server::event_sourcing::envelope::EventEnvelope;
use crate::server::event_sourcing::projector::Projector;

use super::read_model::PipelineConfigurationReadModel;
use super::repository::PipelineConfigurationRepository;

pub struct PipelineConfigurationProjector {
    repository: Arc<dyn PipelineConfigurationRepository>,
}

impl PipelineConfigurationProjector {
    pub const NAME: &'static str = "pipeline_configuration_projector";

    pub fn new(repository: Arc<dyn PipelineConfigurationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl Projector<ConfigurationEvent> for PipelineConfigurationProjector {
    fn name(&self) -> &str {
        Self::NAME
    }

    async fn project(
        &self,
        events: &[EventEnvelope<ConfigurationEvent>],
    ) -> Result<(), AppError> {
        for envelope in events {
            match &envelope.event {
                ConfigurationEvent::PipelineConfigurationCreated(e) => {
                    self.repository
                        .save(PipelineConfigurationReadModel {
                            pipeline_configuration_id: e.pipeline_configuration_id,
                            name: e.name.clone(),
                            embedding_model_id: e.embedding_model_id,
                            generation_model_id: e.generation_model_id,
                            vector_index_id: e.vector_index_id,
                        })
                        .await?;
                }
                ConfigurationEvent::PipelineConfigurationUpdated(e) => {
                    self.repository
                        .save(PipelineConfigurationReadModel {
                            pipeline_configuration_id: e.pipeline_configuration_id,
                            name: e.name.clone(),
                            embedding_model_id: e.embedding_model_id,
                            generation_model_id: e.generation_model_id,
                            vector_index_id: e.vector_index_id,
                        })
                        .await?;
                }
                ConfigurationEvent::PipelineConfigurationDeleted(e) => {
                    self.repository.delete(e.pipeline_configuration_id).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
