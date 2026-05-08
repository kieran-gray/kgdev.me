use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::{
    ai_provider::{
        entity::AiProvdier,
        events::{AiProviderAdded, AiProviderRemoved, AiProviderUpdated},
    },
    configuration::{
        commands::ConfigurationCommand,
        events::{
            ConfigurationCreated, ConfigurationEvent, CurrentEmbeddingModelSet,
            CurrentGenerationModelSet, CurrentVectorIndexSet, EmbeddingModelAdded,
            EmbeddingModelRemoved, EmbeddingModelUpdated, GenerationModelAdded,
            GenerationModelRemoved, GenerationModelUpdated, VectorIndexAdded, VectorIndexRemoved,
            VectorIndexUpdated, VectorStoreProviderAdded, VectorStoreProviderRemoved,
            VectorStoreProviderUpdated,
        },
        exceptions::ConfigurationError,
    },
    embedding_model::entity::EmbeddingModel,
    generation_model::entity::GenerationModel,
    vector_index::entity::VectorIndex,
    vector_store_provider::entity::VectorStoreProvider,
};

// TODO: move elsewhere
pub trait Aggregate: Sized + Clone + Serialize + DeserializeOwned {
    type Event: Clone;
    type Command;
    type Error: std::error::Error;

    fn aggregate_id(&self) -> String;

    fn apply(&mut self, event: &Self::Event);

    fn handle_command(
        state: Option<&Self>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    fn from_events(events: &[Self::Event]) -> Option<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub configuration_id: Uuid,
    pub ai_providers: Vec<AiProvdier>,
    pub vector_store_providers: Vec<VectorStoreProvider>,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
    pub current_embedding_model_id: Option<Uuid>,
    pub current_generation_model_id: Option<Uuid>,
    pub current_vector_index_id: Option<Uuid>,
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
                if self.current_embedding_model_id == Some(e.model_id) {
                    self.current_embedding_model_id = None;
                }
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
                if self.current_generation_model_id == Some(e.model_id) {
                    self.current_generation_model_id = None;
                }
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
                if self.current_vector_index_id == Some(e.index_id) {
                    self.current_vector_index_id = None;
                }
            }
            Self::Event::CurrentEmbeddingModelSet(e) => {
                self.current_embedding_model_id = Some(e.model_id);
            }
            Self::Event::CurrentGenerationModelSet(e) => {
                self.current_generation_model_id = Some(e.model_id);
            }
            Self::Event::CurrentVectorIndexSet(e) => {
                self.current_vector_index_id = Some(e.index_id);
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
            Self::Command::SetCurrentEmbeddingModel(cmd) => {
                let model = Self::find_embedding_model(state, cmd.model_id)?;
                vec![Self::Event::CurrentEmbeddingModelSet(
                    CurrentEmbeddingModelSet {
                        model_id: model.embedding_model_id,
                    },
                )]
            }
            Self::Command::SetCurrentGenerationModel(cmd) => {
                let model = Self::find_generation_model(state, cmd.model_id)?;
                vec![Self::Event::CurrentGenerationModelSet(
                    CurrentGenerationModelSet {
                        model_id: model.generation_model_id,
                    },
                )]
            }
            Self::Command::SetCurrentVectorIndex(cmd) => {
                let index = Self::find_vector_index(state, cmd.index_id)?;
                vec![Self::Event::CurrentVectorIndexSet(CurrentVectorIndexSet {
                    index_id: index.index_id,
                })]
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
            current_embedding_model_id: None,
            current_generation_model_id: None,
            current_vector_index_id: None,
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
    use super::*;
    use crate::server::domain::configuration::commands::{
        AddAiProvider, AddEmbeddingModel, RemoveAiProvider, UpdateEmbeddingModel,
    };

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
}
