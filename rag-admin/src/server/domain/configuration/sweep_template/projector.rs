use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::configuration::events::ConfigurationEvent;
use crate::server::event_sourcing::envelope::EventEnvelope;
use crate::server::event_sourcing::projector::Projector;

use super::read_model::SweepTemplateReadModel;
use super::repository::SweepTemplateRepository;

pub struct SweepTemplateProjector {
    repository: Arc<dyn SweepTemplateRepository>,
}

impl SweepTemplateProjector {
    pub const NAME: &'static str = "sweep_template_projector";

    pub fn new(repository: Arc<dyn SweepTemplateRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl Projector<ConfigurationEvent> for SweepTemplateProjector {
    fn name(&self) -> &str {
        Self::NAME
    }

    async fn project(&self, events: &[EventEnvelope<ConfigurationEvent>]) -> Result<(), AppError> {
        for envelope in events {
            match &envelope.event {
                ConfigurationEvent::SweepTemplateCreated(e) => {
                    self.repository
                        .save(SweepTemplateReadModel {
                            sweep_template_id: e.sweep_template_id,
                            name: e.name.clone(),
                            members: e.members.clone(),
                            is_default: false,
                        })
                        .await?;
                }
                ConfigurationEvent::SweepTemplateUpdated(e) => {
                    self.repository
                        .save(SweepTemplateReadModel {
                            sweep_template_id: e.sweep_template_id,
                            name: e.name.clone(),
                            members: e.members.clone(),
                            is_default: false,
                        })
                        .await?;
                }
                ConfigurationEvent::SweepTemplateDeleted(e) => {
                    self.repository.delete(e.sweep_template_id).await?;
                }
                ConfigurationEvent::SweepTemplateDefaultSet(e) => {
                    self.repository.set_default(e.sweep_template_id).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
