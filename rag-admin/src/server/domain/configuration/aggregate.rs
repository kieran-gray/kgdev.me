use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::{
    aggregate::Aggregate,
    configuration::{
        ai_provider::{AiProvdier, AiProviderAdded, AiProviderRemoved, AiProviderUpdated},
        commands::ConfigurationCommand,
        embedding_model::{
            EmbeddingModel, EmbeddingModelAdded, EmbeddingModelRemoved, EmbeddingModelUpdated,
        },
        events::{ConfigurationCreated, ConfigurationEvent},
        exceptions::ConfigurationError,
        generation_model::{
            GenerationModel, GenerationModelAdded, GenerationModelRemoved, GenerationModelUpdated,
        },
        pipeline_configuration::{
            events::{
                PipelineConfigurationCreated, PipelineConfigurationDeleted,
                PipelineConfigurationUpdated,
            },
            PipelineConfiguration, PipelineConfigurationValidator,
        },
        vector_index::{VectorIndex, VectorIndexAdded, VectorIndexRemoved, VectorIndexUpdated},
        vector_store_provider::{
            VectorStoreProvider, VectorStoreProviderAdded, VectorStoreProviderRemoved,
            VectorStoreProviderUpdated,
        },
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub configuration_id: Uuid,
    pub ai_providers: Vec<AiProvdier>,
    pub vector_store_providers: Vec<VectorStoreProvider>,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
    pub pipeline_configurations: Vec<PipelineConfiguration>,
}

impl Aggregate for Configuration {
    type Event = ConfigurationEvent;
    type Command = ConfigurationCommand;
    type Error = ConfigurationError;

    fn aggregate_id(&self) -> String {
        self.configuration_id.to_string()
    }

    fn apply(&mut self, event: &Self::Event) {
        match event {
            Self::Event::ConfigurationCreated(e) => {
                *self = Self::from_created(e.configuration_id);
            }
            Self::Event::AiProviderAdded(e) => {
                self.ai_providers.push(AiProvdier {
                    provider_id: e.provider_id,
                    name: e.name.clone(),
                });
            }
            Self::Event::AiProviderUpdated(e) => {
                if let Some(provider) = self
                    .ai_providers
                    .iter_mut()
                    .find(|p| p.provider_id == e.provider_id)
                {
                    provider.name = e.name.clone();
                }
            }
            Self::Event::AiProviderRemoved(e) => {
                self.ai_providers.retain(|p| p.provider_id != e.provider_id);
            }
            Self::Event::VectorStoreProviderAdded(e) => {
                self.vector_store_providers.push(VectorStoreProvider {
                    provider_id: e.provider_id,
                    name: e.name.clone(),
                });
            }
            Self::Event::VectorStoreProviderUpdated(e) => {
                if let Some(provider) = self
                    .vector_store_providers
                    .iter_mut()
                    .find(|p| p.provider_id == e.provider_id)
                {
                    provider.name = e.name.clone();
                }
            }
            Self::Event::VectorStoreProviderRemoved(e) => {
                self.vector_store_providers
                    .retain(|p| p.provider_id != e.provider_id);
            }
            Self::Event::EmbeddingModelAdded(e) => {
                self.embedding_models.push(EmbeddingModel {
                    embedding_model_id: e.model_id,
                    provider_id: e.provider_id,
                    model: e.model.clone(),
                    dimensions: e.dimensions,
                });
            }
            Self::Event::EmbeddingModelUpdated(e) => {
                if let Some(embedding_model) = self
                    .embedding_models
                    .iter_mut()
                    .find(|m| m.embedding_model_id == e.model_id)
                {
                    embedding_model.provider_id = e.provider_id;
                    embedding_model.model = e.model.clone();
                    embedding_model.dimensions = e.dimensions;
                }
            }
            Self::Event::EmbeddingModelRemoved(e) => {
                self.embedding_models
                    .retain(|m| m.embedding_model_id != e.model_id);
            }
            Self::Event::GenerationModelAdded(e) => {
                self.generation_models.push(GenerationModel {
                    generation_model_id: e.model_id,
                    provider_id: e.provider_id,
                    model: e.model.clone(),
                });
            }
            Self::Event::GenerationModelUpdated(e) => {
                if let Some(generation_model) = self
                    .generation_models
                    .iter_mut()
                    .find(|m| m.generation_model_id == e.model_id)
                {
                    generation_model.provider_id = e.provider_id;
                    generation_model.model = e.model.clone();
                }
            }
            Self::Event::GenerationModelRemoved(e) => {
                self.generation_models
                    .retain(|m| m.generation_model_id != e.model_id);
            }
            Self::Event::VectorIndexAdded(e) => {
                self.vector_indexes.push(VectorIndex {
                    index_id: e.index_id,
                    vector_store_provider_id: e.vector_store_provider_id,
                    name: e.name.clone(),
                    dimensions: e.dimensions,
                });
            }
            Self::Event::VectorIndexUpdated(e) => {
                if let Some(vector_index) = self
                    .vector_indexes
                    .iter_mut()
                    .find(|v| v.index_id == e.index_id)
                {
                    vector_index.vector_store_provider_id = e.vector_store_provider_id;
                    vector_index.name = e.name.clone();
                    vector_index.dimensions = e.dimensions;
                }
            }
            Self::Event::VectorIndexRemoved(e) => {
                self.vector_indexes.retain(|v| v.index_id != e.index_id);
            }
            Self::Event::PipelineConfigurationCreated(e) => {
                self.pipeline_configurations.push(PipelineConfiguration {
                    pipeline_configuration_id: e.pipeline_configuration_id,
                    name: e.name.clone(),
                    embedding_model_id: e.embedding_model_id,
                    generation_model_id: e.generation_model_id,
                    vector_index_id: e.vector_index_id,
                });
            }
            Self::Event::PipelineConfigurationUpdated(e) => {
                if let Some(pc) = self
                    .pipeline_configurations
                    .iter_mut()
                    .find(|pc| pc.pipeline_configuration_id == e.pipeline_configuration_id)
                {
                    pc.name = e.name.clone();
                    pc.embedding_model_id = e.embedding_model_id;
                    pc.generation_model_id = e.generation_model_id;
                    pc.vector_index_id = e.vector_index_id;
                }
            }
            Self::Event::PipelineConfigurationDeleted(e) => {
                self.pipeline_configurations
                    .retain(|pc| pc.pipeline_configuration_id != e.pipeline_configuration_id);
            }
        }
    }

    fn handle_command(
        state: Option<&Self>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        let mut bootstrap_events = Vec::new();
        let owned_state = match state {
            Some(state) => state.clone(),
            None => {
                let created = ConfigurationCreated {
                    configuration_id: Self::singleton_id(),
                };
                bootstrap_events.push(Self::Event::ConfigurationCreated(created.clone()));
                Self::from_created(created.configuration_id)
            }
        };
        let state = &owned_state;

        let mut events = match command {
            Self::Command::AddAiProvider(cmd) => {
                Self::validate_non_empty("AI provider name", &cmd.name)?;
                Self::ensure_unique_provider_name(state, &cmd.name, None)?;
                vec![Self::Event::AiProviderAdded(AiProviderAdded {
                    provider_id: Uuid::new_v4(),
                    name: cmd.name,
                })]
            }
            Self::Command::UpdateAiProvider(cmd) => {
                let provider = Self::find_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("AI provider name", &cmd.name)?;
                Self::ensure_unique_provider_name(state, &cmd.name, Some(provider.provider_id))?;
                vec![Self::Event::AiProviderUpdated(AiProviderUpdated {
                    provider_id: provider.provider_id,
                    name: cmd.name,
                })]
            }
            Self::Command::RemoveAiProvider(cmd) => {
                let provider = Self::find_provider(state, cmd.provider_id)?;
                if state
                    .embedding_models
                    .iter()
                    .any(|model| model.provider_id == provider.provider_id)
                {
                    return Err(Self::Error::ValidationError(format!(
                        "AI provider {} cannot be removed while embedding models reference it",
                        provider.name
                    )));
                }
                if state
                    .generation_models
                    .iter()
                    .any(|model| model.provider_id == provider.provider_id)
                {
                    return Err(Self::Error::ValidationError(format!(
                        "AI provider {} cannot be removed while generation models reference it",
                        provider.name
                    )));
                }
                vec![Self::Event::AiProviderRemoved(AiProviderRemoved {
                    provider_id: provider.provider_id,
                })]
            }

            Self::Command::AddEmbeddingModel(cmd) => {
                Self::find_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("embedding model", &cmd.model)?;
                Self::validate_positive("embedding dimensions", cmd.dimensions)?;
                Self::ensure_unique_embedding_model(state, cmd.provider_id, &cmd.model, None)?;
                vec![Self::Event::EmbeddingModelAdded(EmbeddingModelAdded {
                    model_id: Uuid::new_v4(),
                    provider_id: cmd.provider_id,
                    model: cmd.model,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::UpdateEmbeddingModel(cmd) => {
                let model = Self::find_embedding_model(state, cmd.model_id)?;
                Self::find_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("embedding model", &cmd.model)?;
                Self::validate_positive("embedding dimensions", cmd.dimensions)?;
                Self::ensure_unique_embedding_model(
                    state,
                    cmd.provider_id,
                    &cmd.model,
                    Some(model.embedding_model_id),
                )?;
                vec![Self::Event::EmbeddingModelUpdated(EmbeddingModelUpdated {
                    model_id: model.embedding_model_id,
                    provider_id: cmd.provider_id,
                    model: cmd.model,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::RemoveEmbeddingModel(cmd) => {
                let model = Self::find_embedding_model(state, cmd.model_id)?;
                vec![Self::Event::EmbeddingModelRemoved(EmbeddingModelRemoved {
                    model_id: model.embedding_model_id,
                })]
            }

            Self::Command::AddGenerationModel(cmd) => {
                Self::find_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("generation model", &cmd.model)?;
                Self::ensure_unique_generation_model(state, cmd.provider_id, &cmd.model, None)?;
                vec![Self::Event::GenerationModelAdded(GenerationModelAdded {
                    model_id: Uuid::new_v4(),
                    provider_id: cmd.provider_id,
                    model: cmd.model,
                })]
            }
            Self::Command::UpdateGenerationModel(cmd) => {
                let model = Self::find_generation_model(state, cmd.model_id)?;
                Self::find_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("generation model", &cmd.model)?;
                Self::ensure_unique_generation_model(
                    state,
                    cmd.provider_id,
                    &cmd.model,
                    Some(model.generation_model_id),
                )?;
                vec![Self::Event::GenerationModelUpdated(
                    GenerationModelUpdated {
                        model_id: model.generation_model_id,
                        provider_id: cmd.provider_id,
                        model: cmd.model,
                    },
                )]
            }
            Self::Command::RemoveGenerationModel(cmd) => {
                let model = Self::find_generation_model(state, cmd.model_id)?;
                vec![Self::Event::GenerationModelRemoved(
                    GenerationModelRemoved {
                        model_id: model.generation_model_id,
                    },
                )]
            }

            Self::Command::AddVectorStoreProvider(cmd) => {
                Self::validate_non_empty("vector store provider name", &cmd.name)?;
                Self::ensure_unique_vector_store_provider_name(state, &cmd.name, None)?;
                vec![Self::Event::VectorStoreProviderAdded(
                    VectorStoreProviderAdded {
                        provider_id: Uuid::new_v4(),
                        name: cmd.name,
                    },
                )]
            }
            Self::Command::UpdateVectorStoreProvider(cmd) => {
                let provider = Self::find_vector_store_provider(state, cmd.provider_id)?;
                Self::validate_non_empty("vector store provider name", &cmd.name)?;
                Self::ensure_unique_vector_store_provider_name(
                    state,
                    &cmd.name,
                    Some(provider.provider_id),
                )?;
                vec![Self::Event::VectorStoreProviderUpdated(
                    VectorStoreProviderUpdated {
                        provider_id: provider.provider_id,
                        name: cmd.name,
                    },
                )]
            }
            Self::Command::RemoveVectorStoreProvider(cmd) => {
                let provider = Self::find_vector_store_provider(state, cmd.provider_id)?;
                if state
                    .vector_indexes
                    .iter()
                    .any(|index| index.vector_store_provider_id == provider.provider_id)
                {
                    return Err(Self::Error::ValidationError(format!(
                        "Vector store provider {} cannot be removed while indexes reference it",
                        provider.name
                    )));
                }
                vec![Self::Event::VectorStoreProviderRemoved(
                    VectorStoreProviderRemoved {
                        provider_id: provider.provider_id,
                    },
                )]
            }
            Self::Command::AddVectorIndex(cmd) => {
                Self::find_vector_store_provider(state, cmd.vector_store_provider_id)?;
                Self::validate_non_empty("vector index name", &cmd.name)?;
                Self::validate_positive("vector index dimensions", cmd.dimensions)?;
                Self::ensure_unique_vector_index_name(state, &cmd.name, None)?;
                vec![Self::Event::VectorIndexAdded(VectorIndexAdded {
                    index_id: Uuid::new_v4(),
                    vector_store_provider_id: cmd.vector_store_provider_id,
                    name: cmd.name,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::UpdateVectorIndex(cmd) => {
                let index = Self::find_vector_index(state, cmd.index_id)?;
                Self::find_vector_store_provider(state, cmd.vector_store_provider_id)?;
                Self::validate_non_empty("vector index name", &cmd.name)?;
                Self::validate_positive("vector index dimensions", cmd.dimensions)?;
                Self::ensure_unique_vector_index_name(state, &cmd.name, Some(index.index_id))?;
                vec![Self::Event::VectorIndexUpdated(VectorIndexUpdated {
                    index_id: index.index_id,
                    vector_store_provider_id: cmd.vector_store_provider_id,
                    name: cmd.name,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::RemoveVectorIndex(cmd) => {
                let index = Self::find_vector_index(state, cmd.index_id)?;
                vec![Self::Event::VectorIndexRemoved(VectorIndexRemoved {
                    index_id: index.index_id,
                })]
            }

            Self::Command::CreatePipelineConfiguration(cmd) => {
                Self::validate_non_empty("pipeline configuration name", &cmd.name)?;
                let embedding_model = Self::find_embedding_model(state, cmd.embedding_model_id)?;
                Self::find_generation_model(state, cmd.generation_model_id)?;
                let vector_index = Self::find_vector_index(state, cmd.vector_index_id)?;
                PipelineConfigurationValidator::validate_combination(
                    embedding_model,
                    vector_index,
                )?;
                vec![Self::Event::PipelineConfigurationCreated(
                    PipelineConfigurationCreated {
                        pipeline_configuration_id: Uuid::new_v4(),
                        name: cmd.name,
                        embedding_model_id: cmd.embedding_model_id,
                        generation_model_id: cmd.generation_model_id,
                        vector_index_id: cmd.vector_index_id,
                    },
                )]
            }

            Self::Command::UpdatePipelineConfiguration(cmd) => {
                Self::find_pipeline_configuration(state, cmd.pipeline_configuration_id)?;
                Self::validate_non_empty("pipeline configuration name", &cmd.name)?;
                let embedding_model = Self::find_embedding_model(state, cmd.embedding_model_id)?;
                Self::find_generation_model(state, cmd.generation_model_id)?;
                let vector_index = Self::find_vector_index(state, cmd.vector_index_id)?;
                PipelineConfigurationValidator::validate_combination(
                    embedding_model,
                    vector_index,
                )?;
                vec![Self::Event::PipelineConfigurationUpdated(
                    PipelineConfigurationUpdated {
                        pipeline_configuration_id: cmd.pipeline_configuration_id,
                        name: cmd.name,
                        embedding_model_id: cmd.embedding_model_id,
                        generation_model_id: cmd.generation_model_id,
                        vector_index_id: cmd.vector_index_id,
                    },
                )]
            }

            Self::Command::DeletePipelineConfiguration(cmd) => {
                Self::find_pipeline_configuration(state, cmd.pipeline_configuration_id)?;
                vec![Self::Event::PipelineConfigurationDeleted(
                    PipelineConfigurationDeleted {
                        pipeline_configuration_id: cmd.pipeline_configuration_id,
                    },
                )]
            }
        };
        bootstrap_events.append(&mut events);
        Ok(bootstrap_events)
    }

    fn from_events(events: &[Self::Event]) -> Option<Self> {
        let mut state = None;

        for event in events {
            match (&mut state, event) {
                (None, Self::Event::ConfigurationCreated(created)) => {
                    state = Some(Self::from_created(created.configuration_id));
                }
                (Some(_), Self::Event::ConfigurationCreated(_)) => return None,
                (None, _) => return None,
                (Some(configuration), event) => configuration.apply(event),
            }
        }

        state
    }
}

impl Configuration {
    pub fn singleton_id() -> Uuid {
        Uuid::nil()
    }

    fn from_created(configuration_id: Uuid) -> Self {
        Self {
            configuration_id,
            ai_providers: Vec::new(),
            vector_store_providers: Vec::new(),
            embedding_models: Vec::new(),
            generation_models: Vec::new(),
            vector_indexes: Vec::new(),
            pipeline_configurations: Vec::new(),
        }
    }

    fn validate_non_empty(field: &str, value: &str) -> Result<(), ConfigurationError> {
        if value.trim().is_empty() {
            return Err(ConfigurationError::ValidationError(format!(
                "{field} cannot be empty"
            )));
        }
        Ok(())
    }

    fn validate_positive(field: &str, value: u32) -> Result<(), ConfigurationError> {
        if value == 0 {
            return Err(ConfigurationError::ValidationError(format!(
                "{field} must be greater than zero"
            )));
        }
        Ok(())
    }

    fn find_vector_store_provider(
        &self,
        provider_id: Uuid,
    ) -> Result<&VectorStoreProvider, ConfigurationError> {
        self.vector_store_providers
            .iter()
            .find(|p| p.provider_id == provider_id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn find_provider(&self, provider_id: Uuid) -> Result<&AiProvdier, ConfigurationError> {
        self.ai_providers
            .iter()
            .find(|provider| provider.provider_id == provider_id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn find_embedding_model(&self, model_id: Uuid) -> Result<&EmbeddingModel, ConfigurationError> {
        self.embedding_models
            .iter()
            .find(|model| model.embedding_model_id == model_id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn find_generation_model(
        &self,
        model_id: Uuid,
    ) -> Result<&GenerationModel, ConfigurationError> {
        self.generation_models
            .iter()
            .find(|model| model.generation_model_id == model_id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn find_vector_index(&self, index_id: Uuid) -> Result<&VectorIndex, ConfigurationError> {
        self.vector_indexes
            .iter()
            .find(|index| index.index_id == index_id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn find_pipeline_configuration(
        &self,
        id: Uuid,
    ) -> Result<&PipelineConfiguration, ConfigurationError> {
        self.pipeline_configurations
            .iter()
            .find(|pc| pc.pipeline_configuration_id == id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn ensure_unique_vector_store_provider_name(
        &self,
        name: &str,
        provider_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self
            .vector_store_providers
            .iter()
            .any(|p| p.name == name && Some(p.provider_id) != provider_id)
        {
            return Err(ConfigurationError::ValidationError(format!(
                "Vector store provider with name {name} already exists"
            )));
        }
        Ok(())
    }

    fn ensure_unique_provider_name(
        &self,
        name: &str,
        provider_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self
            .ai_providers
            .iter()
            .any(|provider| provider.name == name && Some(provider.provider_id) != provider_id)
        {
            return Err(ConfigurationError::ValidationError(format!(
                "AI provider with name {name} already exists"
            )));
        }
        Ok(())
    }

    fn ensure_unique_embedding_model(
        &self,
        provider_id: Uuid,
        model_name: &str,
        model_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self.embedding_models.iter().any(|model| {
            model.provider_id == provider_id
                && model.model == model_name
                && Some(model.embedding_model_id) != model_id
        }) {
            return Err(ConfigurationError::ValidationError(format!(
                "Embedding model {model_name} already exists for provider {provider_id}"
            )));
        }
        Ok(())
    }

    fn ensure_unique_generation_model(
        &self,
        provider_id: Uuid,
        model_name: &str,
        model_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self.generation_models.iter().any(|model| {
            model.provider_id == provider_id
                && model.model == model_name
                && Some(model.generation_model_id) != model_id
        }) {
            return Err(ConfigurationError::ValidationError(format!(
                "Generation model {model_name} already exists for provider {provider_id}"
            )));
        }
        Ok(())
    }

    fn ensure_unique_vector_index_name(
        &self,
        name: &str,
        index_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self
            .vector_indexes
            .iter()
            .any(|index| index.name == name && Some(index.index_id) != index_id)
        {
            return Err(ConfigurationError::ValidationError(format!(
                "Vector index with name {name} already exists"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::server::domain::configuration::{
        ai_provider::{AddAiProvider, RemoveAiProvider},
        embedding_model::{AddEmbeddingModel, UpdateEmbeddingModel},
        pipeline_configuration::commands::CreatePipelineConfiguration,
    };

    use super::*;

    #[test]
    fn first_command_bootstraps_configuration() {
        let events = Configuration::handle_command(
            None,
            ConfigurationCommand::AddAiProvider(AddAiProvider {
                name: "OpenAI".into(),
            }),
        )
        .unwrap();

        assert!(matches!(
            &events[0],
            ConfigurationEvent::ConfigurationCreated(_)
        ));
        assert!(matches!(&events[1], ConfigurationEvent::AiProviderAdded(_)));

        let configuration = Configuration::from_events(&events).unwrap();
        assert_eq!(
            configuration.configuration_id,
            Configuration::singleton_id()
        );
        assert_eq!(configuration.ai_providers.len(), 1);
        assert_eq!(configuration.ai_providers[0].name, "OpenAI");
    }

    #[test]
    fn cannot_add_embedding_model_for_unknown_provider() {
        let error = Configuration::handle_command(
            Some(&Configuration::from_created(Configuration::singleton_id())),
            ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                provider_id: Uuid::new_v4(),
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigurationError::NotFound));
    }

    #[test]
    fn cannot_remove_provider_that_is_still_referenced() {
        let provider_id = Uuid::new_v4();
        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id: Uuid::new_v4(),
                provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
        ])
        .unwrap();

        let error = Configuration::handle_command(
            Some(&configuration),
            ConfigurationCommand::RemoveAiProvider(RemoveAiProvider { provider_id }),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigurationError::ValidationError(_)));
    }

    #[test]
    fn replay_requires_configuration_created_as_first_event() {
        let configuration =
            Configuration::from_events(&[ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id: Uuid::new_v4(),
                name: "OpenAI".into(),
            })]);

        assert!(configuration.is_none());
    }

    #[test]
    fn can_move_embedding_model_to_another_provider() {
        let first_provider_id = Uuid::new_v4();
        let second_provider_id = Uuid::new_v4();
        let model_id = Uuid::new_v4();
        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id: first_provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id: second_provider_id,
                name: "Anthropic".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id,
                provider_id: first_provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
        ])
        .unwrap();

        let events = Configuration::handle_command(
            Some(&configuration),
            ConfigurationCommand::UpdateEmbeddingModel(UpdateEmbeddingModel {
                model_id,
                provider_id: second_provider_id,
                model: "voyage-3-lite".into(),
                dimensions: 1024,
            }),
        )
        .unwrap();

        assert_eq!(
            events,
            vec![ConfigurationEvent::EmbeddingModelUpdated(
                EmbeddingModelUpdated {
                    model_id,
                    provider_id: second_provider_id,
                    model: "voyage-3-lite".into(),
                    dimensions: 1024,
                }
            )]
        );
    }

    #[test]
    fn create_pipeline_configuration_validates_dimensions_match() {
        let provider_id = Uuid::new_v4();
        let vs_provider_id = Uuid::new_v4();
        let embedding_model_id = Uuid::new_v4();
        let generation_model_id = Uuid::new_v4();
        let vector_index_id = Uuid::new_v4();

        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id: embedding_model_id,
                provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
            ConfigurationEvent::GenerationModelAdded(GenerationModelAdded {
                model_id: generation_model_id,
                provider_id,
                model: "gpt-4o".into(),
            }),
            ConfigurationEvent::VectorStoreProviderAdded(VectorStoreProviderAdded {
                provider_id: vs_provider_id,
                name: "Cloudflare".into(),
            }),
            ConfigurationEvent::VectorIndexAdded(VectorIndexAdded {
                index_id: vector_index_id,
                vector_store_provider_id: vs_provider_id,
                name: "my-index".into(),
                dimensions: 1024, // mismatch: embedding is 1536
            }),
        ])
        .unwrap();

        let error = Configuration::handle_command(
            Some(&configuration),
            ConfigurationCommand::CreatePipelineConfiguration(CreatePipelineConfiguration {
                name: "production".into(),
                embedding_model_id,
                generation_model_id,
                vector_index_id,
            }),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigurationError::ValidationError(_)));
    }

    #[test]
    fn create_pipeline_configuration_succeeds_with_matching_dimensions() {
        let provider_id = Uuid::new_v4();
        let vs_provider_id = Uuid::new_v4();
        let embedding_model_id = Uuid::new_v4();
        let generation_model_id = Uuid::new_v4();
        let vector_index_id = Uuid::new_v4();

        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::AiProviderAdded(AiProviderAdded {
                provider_id,
                name: "OpenAI".into(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id: embedding_model_id,
                provider_id,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
            ConfigurationEvent::GenerationModelAdded(GenerationModelAdded {
                model_id: generation_model_id,
                provider_id,
                model: "gpt-4o".into(),
            }),
            ConfigurationEvent::VectorStoreProviderAdded(VectorStoreProviderAdded {
                provider_id: vs_provider_id,
                name: "Cloudflare".into(),
            }),
            ConfigurationEvent::VectorIndexAdded(VectorIndexAdded {
                index_id: vector_index_id,
                vector_store_provider_id: vs_provider_id,
                name: "my-index".into(),
                dimensions: 1536,
            }),
        ])
        .unwrap();

        let events = Configuration::handle_command(
            Some(&configuration),
            ConfigurationCommand::CreatePipelineConfiguration(CreatePipelineConfiguration {
                name: "production".into(),
                embedding_model_id,
                generation_model_id,
                vector_index_id,
            }),
        )
        .unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ConfigurationEvent::PipelineConfigurationCreated(_)
        ));
    }
}
