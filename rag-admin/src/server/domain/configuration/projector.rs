use std::sync::Arc;

use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::configuration::{
    embedding_model::entity::EmbeddingModel, events::ConfigurationEvent,
    generation_model::entity::GenerationModel, read_model::ConfigurationReadModel,
    repository::ConfigurationRepository, vector_index::entity::VectorIndex,
};
use crate::server::event_sourcing::envelope::EventEnvelope;
use crate::server::event_sourcing::projector::Projector;

pub struct ConfigurationProjector {
    repository: Arc<dyn ConfigurationRepository>,
}

impl ConfigurationProjector {
    pub const NAME: &'static str = "configuration_projector";

    pub fn new(repository: Arc<dyn ConfigurationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl Projector<ConfigurationEvent> for ConfigurationProjector {
    fn name(&self) -> &str {
        Self::NAME
    }

    async fn project(
        &self,
        events: &[EventEnvelope<ConfigurationEvent>],
    ) -> Result<(), AppError> {
        let mut model = self.repository.load().await?;
        let mut changed = false;
        for envelope in events {
            if apply(&mut model, &envelope.event) {
                changed = true;
            }
        }
        if changed {
            self.repository.save(model).await?;
        }
        Ok(())
    }
}

fn apply(model: &mut ConfigurationReadModel, event: &ConfigurationEvent) -> bool {
    match event {
        ConfigurationEvent::ConfigurationCreated(e) => {
            model.configuration_id = e.configuration_id;
            model.embedding_models.clear();
            model.generation_models.clear();
            model.vector_indexes.clear();
            true
        }
        ConfigurationEvent::EmbeddingModelAdded(e) => {
            model.embedding_models.push(EmbeddingModel {
                embedding_model_id: e.model_id,
                kind: e.kind,
                model: e.model.clone(),
                dimensions: e.dimensions,
            });
            true
        }
        ConfigurationEvent::EmbeddingModelUpdated(e) => {
            if let Some(m) = model
                .embedding_models
                .iter_mut()
                .find(|m| m.embedding_model_id == e.model_id)
            {
                m.kind = e.kind;
                m.model = e.model.clone();
                m.dimensions = e.dimensions;
            }
            true
        }
        ConfigurationEvent::EmbeddingModelRemoved(e) => {
            model
                .embedding_models
                .retain(|m| m.embedding_model_id != e.model_id);
            true
        }
        ConfigurationEvent::GenerationModelAdded(e) => {
            model.generation_models.push(GenerationModel {
                generation_model_id: e.model_id,
                kind: e.kind,
                model: e.model.clone(),
            });
            true
        }
        ConfigurationEvent::GenerationModelUpdated(e) => {
            if let Some(m) = model
                .generation_models
                .iter_mut()
                .find(|m| m.generation_model_id == e.model_id)
            {
                m.kind = e.kind;
                m.model = e.model.clone();
            }
            true
        }
        ConfigurationEvent::GenerationModelRemoved(e) => {
            model
                .generation_models
                .retain(|m| m.generation_model_id != e.model_id);
            true
        }
        ConfigurationEvent::VectorIndexAdded(e) => {
            model.vector_indexes.push(VectorIndex {
                index_id: e.index_id,
                kind: e.kind,
                name: e.name.clone(),
                dimensions: e.dimensions,
            });
            true
        }
        ConfigurationEvent::VectorIndexUpdated(e) => {
            if let Some(i) = model
                .vector_indexes
                .iter_mut()
                .find(|i| i.index_id == e.index_id)
            {
                i.kind = e.kind;
                i.name = e.name.clone();
                i.dimensions = e.dimensions;
            }
            true
        }
        ConfigurationEvent::VectorIndexRemoved(e) => {
            model.vector_indexes.retain(|i| i.index_id != e.index_id);
            true
        }
        ConfigurationEvent::PipelineConfigurationCreated(_)
        | ConfigurationEvent::PipelineConfigurationUpdated(_)
        | ConfigurationEvent::PipelineConfigurationDeleted(_)
        | ConfigurationEvent::ChunkingConfigurationCreated(_)
        | ConfigurationEvent::ChunkingConfigurationUpdated(_)
        | ConfigurationEvent::ChunkingConfigurationDeleted(_) => false,
    }
}
