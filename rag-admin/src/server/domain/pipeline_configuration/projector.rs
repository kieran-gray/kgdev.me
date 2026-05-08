use crate::server::domain::configuration::{
    events::ConfigurationEvent, exceptions::ConfigurationError,
};
use crate::server::domain::pipeline_configuration::PipelineConfiguration;

pub struct PipelineConfigurationProjector;

impl PipelineConfigurationProjector {
    pub fn project(
        events: &[ConfigurationEvent],
    ) -> Result<PipelineConfiguration, ConfigurationError> {
        let mut current = PipelineConfiguration::default();

        for event in events {
            Self::apply(&mut current, event)?;
        }

        Ok(current)
    }

    pub fn apply(
        current: &mut PipelineConfiguration,
        event: &ConfigurationEvent,
    ) -> Result<(), ConfigurationError> {
        match event {
            ConfigurationEvent::ConfigurationCreated(e) => {
                current.configuration_id = e.configuration_id;
            }
            ConfigurationEvent::AiProviderAdded(e) => {
                current
                    .ai_providers
                    .push(crate::server::domain::ai_provider::entity::AiProvdier {
                        provider_id: e.provider_id,
                        name: e.name.clone(),
                    });
            }
            ConfigurationEvent::AiProviderUpdated(e) => {
                let provider = current
                    .ai_providers
                    .iter_mut()
                    .find(|provider| provider.provider_id == e.provider_id)
                    .ok_or_else(|| {
                        ConfigurationError::InvalidEvent(format!(
                            "cannot update missing AI provider {}",
                            e.provider_id
                        ))
                    })?;
                provider.name = e.name.clone();
            }
            ConfigurationEvent::AiProviderRemoved(e) => {
                current
                    .ai_providers
                    .retain(|provider| provider.provider_id != e.provider_id);
            }
            ConfigurationEvent::VectorStoreProviderAdded(e) => {
                current.vector_store_providers.push(
                    crate::server::domain::vector_store_provider::entity::VectorStoreProvider {
                        provider_id: e.provider_id,
                        name: e.name.clone(),
                    },
                );
            }
            ConfigurationEvent::VectorStoreProviderUpdated(e) => {
                let provider = current
                    .vector_store_providers
                    .iter_mut()
                    .find(|p| p.provider_id == e.provider_id)
                    .ok_or_else(|| {
                        ConfigurationError::InvalidEvent(format!(
                            "cannot update missing vector store provider {}",
                            e.provider_id
                        ))
                    })?;
                provider.name = e.name.clone();
            }
            ConfigurationEvent::VectorStoreProviderRemoved(e) => {
                current
                    .vector_store_providers
                    .retain(|p| p.provider_id != e.provider_id);
            }
            ConfigurationEvent::EmbeddingModelAdded(e) => {
                current.embedding_models.push(
                    crate::server::domain::embedding_model::entity::EmbeddingModel {
                        embedding_model_id: e.model_id,
                        provider_id: e.provider_id,
                        model: e.model.clone(),
                        dimensions: e.dimensions,
                    },
                );
            }
            ConfigurationEvent::EmbeddingModelUpdated(e) => {
                let model = current
                    .embedding_models
                    .iter_mut()
                    .find(|model| model.embedding_model_id == e.model_id)
                    .ok_or_else(|| {
                        ConfigurationError::InvalidEvent(format!(
                            "cannot update missing embedding model {}",
                            e.model_id
                        ))
                    })?;
                model.provider_id = e.provider_id;
                model.model = e.model.clone();
                model.dimensions = e.dimensions;
            }
            ConfigurationEvent::EmbeddingModelRemoved(e) => {
                current
                    .embedding_models
                    .retain(|model| model.embedding_model_id != e.model_id);
                if current.current_embedding_model_id == Some(e.model_id) {
                    current.current_embedding_model_id = None;
                }
            }
            ConfigurationEvent::GenerationModelAdded(e) => {
                current.generation_models.push(
                    crate::server::domain::generation_model::entity::GenerationModel {
                        generation_model_id: e.model_id,
                        provider_id: e.provider_id,
                        model: e.model.clone(),
                    },
                );
            }
            ConfigurationEvent::GenerationModelUpdated(e) => {
                let model = current
                    .generation_models
                    .iter_mut()
                    .find(|model| model.generation_model_id == e.model_id)
                    .ok_or_else(|| {
                        ConfigurationError::InvalidEvent(format!(
                            "cannot update missing generation model {}",
                            e.model_id
                        ))
                    })?;
                model.provider_id = e.provider_id;
                model.model = e.model.clone();
            }
            ConfigurationEvent::GenerationModelRemoved(e) => {
                current
                    .generation_models
                    .retain(|model| model.generation_model_id != e.model_id);
                if current.current_generation_model_id == Some(e.model_id) {
                    current.current_generation_model_id = None;
                }
            }
            ConfigurationEvent::VectorIndexAdded(e) => {
                current.vector_indexes.push(
                    crate::server::domain::vector_index::entity::VectorIndex {
                        index_id: e.index_id,
                        vector_store_provider_id: e.vector_store_provider_id,
                        name: e.name.clone(),
                        dimensions: e.dimensions,
                    },
                );
            }
            ConfigurationEvent::VectorIndexUpdated(e) => {
                let index = current
                    .vector_indexes
                    .iter_mut()
                    .find(|index| index.index_id == e.index_id)
                    .ok_or_else(|| {
                        ConfigurationError::InvalidEvent(format!(
                            "cannot update missing vector index {}",
                            e.index_id
                        ))
                    })?;
                index.vector_store_provider_id = e.vector_store_provider_id;
                index.name = e.name.clone();
                index.dimensions = e.dimensions;
            }
            ConfigurationEvent::VectorIndexRemoved(e) => {
                current
                    .vector_indexes
                    .retain(|index| index.index_id != e.index_id);
                if current.current_vector_index_id == Some(e.index_id) {
                    current.current_vector_index_id = None;
                }
            }
            ConfigurationEvent::CurrentEmbeddingModelSet(e) => {
                if current
                    .embedding_models
                    .iter()
                    .all(|model| model.embedding_model_id != e.model_id)
                {
                    return Err(ConfigurationError::InvalidEvent(format!(
                        "cannot set missing embedding model {} as current",
                        e.model_id
                    )));
                }
                current.current_embedding_model_id = Some(e.model_id);
            }
            ConfigurationEvent::CurrentGenerationModelSet(e) => {
                if current
                    .generation_models
                    .iter()
                    .all(|model| model.generation_model_id != e.model_id)
                {
                    return Err(ConfigurationError::InvalidEvent(format!(
                        "cannot set missing generation model {} as current",
                        e.model_id
                    )));
                }
                current.current_generation_model_id = Some(e.model_id);
            }
            ConfigurationEvent::CurrentVectorIndexSet(e) => {
                if current
                    .vector_indexes
                    .iter()
                    .all(|index| index.index_id != e.index_id)
                {
                    return Err(ConfigurationError::InvalidEvent(format!(
                        "cannot set missing vector index {} as current",
                        e.index_id
                    )));
                }
                current.current_vector_index_id = Some(e.index_id);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::server::domain::configuration::events::{
        AiProviderAdded, ConfigurationCreated, CurrentEmbeddingModelSet, EmbeddingModelAdded,
        EmbeddingModelUpdated,
    };

    #[test]
    fn project_sets_current_entities() {
        let provider_id = Uuid::new_v4();
        let model_id = Uuid::new_v4();
        let current = PipelineConfigurationProjector::project(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Uuid::nil(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id,
                provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
            ConfigurationEvent::CurrentEmbeddingModelSet(CurrentEmbeddingModelSet { model_id }),
        ])
        .unwrap();

        assert_eq!(current.current_embedding_model_id, Some(model_id));
        assert_eq!(current.current_embedding_provider().unwrap().name, "OpenAI");
        assert_eq!(
            current.current_embedding_model().unwrap().model,
            "text-embedding-3-small"
        );
    }

    #[test]
    fn project_updates_embedding_model_provider() {
        let provider_id = Uuid::new_v4();
        let other_provider_id = Uuid::new_v4();
        let model_id = Uuid::new_v4();
        let current = PipelineConfigurationProjector::project(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Uuid::nil(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id: other_provider_id,
                name: "Anthropic".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id,
                provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
            ConfigurationEvent::EmbeddingModelUpdated(EmbeddingModelUpdated {
                model_id,
                provider_id: other_provider_id,
                model: "voyage-3-lite".into(),
                dimensions: 1024,
            }),
        ])
        .unwrap();

        let model = current.embedding_models.first().unwrap();
        assert_eq!(model.provider_id, other_provider_id);
        assert_eq!(model.model, "voyage-3-lite");
        assert_eq!(model.dimensions, 1024);
    }
}
