use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::{
    domain::configuration::{
        chunking_configuration::{
            events::{
                ChunkingConfigurationCreated, ChunkingConfigurationDeleted,
                ChunkingConfigurationUpdated,
            },
            ChunkingConfiguration,
        },
        commands::ConfigurationCommand,
        embedding_model::{
            EmbeddingModel, EmbeddingModelAdded, EmbeddingModelRemoved, EmbeddingModelUpdated,
        },
        events::{ConfigurationCreated, ConfigurationEvent},
        exceptions::ConfigurationError,
        generation_model::{
            GenerationModel, GenerationModelAdded, GenerationModelRemoved, GenerationModelUpdated,
        },
        kinds::AiProviderKind,
        pipeline_configuration::{
            events::{
                PipelineConfigurationCreated, PipelineConfigurationDeleted,
                PipelineConfigurationUpdated,
            },
            PipelineConfiguration, PipelineConfigurationValidator,
        },
        sweep_template::{
            events::{
                SweepTemplateCreated, SweepTemplateDefaultSet, SweepTemplateDeleted,
                SweepTemplateUpdated,
            },
            SweepTemplate,
        },
        vector_index::{VectorIndex, VectorIndexAdded, VectorIndexRemoved, VectorIndexUpdated},
    },
    event_sourcing::Aggregate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub configuration_id: Uuid,
    pub embedding_models: Vec<EmbeddingModel>,
    pub generation_models: Vec<GenerationModel>,
    pub vector_indexes: Vec<VectorIndex>,
    pub pipeline_configurations: Vec<PipelineConfiguration>,
    pub chunking_configurations: Vec<ChunkingConfiguration>,
    pub sweep_templates: Vec<SweepTemplate>,
    pub default_sweep_template_id: Option<Uuid>,
}

impl Aggregate for Configuration {
    type Event = ConfigurationEvent;
    type Command = ConfigurationCommand;
    type Error = ConfigurationError;

    fn aggregate_type() -> &'static str {
        "configuration"
    }

    fn apply(&mut self, event: &Self::Event) {
        match event {
            Self::Event::ConfigurationCreated(e) => {
                *self = Self::from_created(e.configuration_id);
            }
            Self::Event::EmbeddingModelAdded(e) => {
                self.embedding_models.push(EmbeddingModel {
                    embedding_model_id: e.model_id,
                    kind: e.kind,
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
                    embedding_model.kind = e.kind;
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
                    kind: e.kind,
                    model: e.model.clone(),
                });
            }
            Self::Event::GenerationModelUpdated(e) => {
                if let Some(generation_model) = self
                    .generation_models
                    .iter_mut()
                    .find(|m| m.generation_model_id == e.model_id)
                {
                    generation_model.kind = e.kind;
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
                    kind: e.kind,
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
                    vector_index.kind = e.kind;
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
            Self::Event::ChunkingConfigurationCreated(e) => {
                self.chunking_configurations.push(ChunkingConfiguration {
                    chunking_configuration_id: e.chunking_configuration_id,
                    name: e.name.clone(),
                    config: e.config,
                });
            }
            Self::Event::ChunkingConfigurationUpdated(e) => {
                if let Some(cc) = self
                    .chunking_configurations
                    .iter_mut()
                    .find(|cc| cc.chunking_configuration_id == e.chunking_configuration_id)
                {
                    cc.name = e.name.clone();
                    cc.config = e.config;
                }
            }
            Self::Event::ChunkingConfigurationDeleted(e) => {
                self.chunking_configurations
                    .retain(|cc| cc.chunking_configuration_id != e.chunking_configuration_id);
            }
            Self::Event::SweepTemplateCreated(e) => {
                self.sweep_templates.push(SweepTemplate {
                    sweep_template_id: e.sweep_template_id,
                    name: e.name.clone(),
                    members: e.members.clone(),
                });
            }
            Self::Event::SweepTemplateUpdated(e) => {
                if let Some(st) = self
                    .sweep_templates
                    .iter_mut()
                    .find(|st| st.sweep_template_id == e.sweep_template_id)
                {
                    st.name = e.name.clone();
                    st.members = e.members.clone();
                }
            }
            Self::Event::SweepTemplateDeleted(e) => {
                self.sweep_templates
                    .retain(|st| st.sweep_template_id != e.sweep_template_id);
                if self.default_sweep_template_id == Some(e.sweep_template_id) {
                    self.default_sweep_template_id = None;
                }
            }
            Self::Event::SweepTemplateDefaultSet(e) => {
                self.default_sweep_template_id = Some(e.sweep_template_id);
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
            Self::Command::AddEmbeddingModel(cmd) => {
                Self::validate_non_empty("embedding model", &cmd.model)?;
                Self::validate_positive("embedding dimensions", cmd.dimensions)?;
                Self::validate_model_id_format(cmd.kind, &cmd.model)?;
                Self::ensure_unique_embedding_model(state, cmd.kind, &cmd.model, None)?;
                vec![Self::Event::EmbeddingModelAdded(EmbeddingModelAdded {
                    model_id: Uuid::new_v4(),
                    kind: cmd.kind,
                    model: cmd.model,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::UpdateEmbeddingModel(cmd) => {
                let model = Self::find_embedding_model(state, cmd.model_id)?;
                Self::validate_non_empty("embedding model", &cmd.model)?;
                Self::validate_positive("embedding dimensions", cmd.dimensions)?;
                Self::validate_model_id_format(cmd.kind, &cmd.model)?;
                Self::ensure_unique_embedding_model(
                    state,
                    cmd.kind,
                    &cmd.model,
                    Some(model.embedding_model_id),
                )?;
                vec![Self::Event::EmbeddingModelUpdated(EmbeddingModelUpdated {
                    model_id: model.embedding_model_id,
                    kind: cmd.kind,
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
                Self::validate_non_empty("generation model", &cmd.model)?;
                Self::validate_model_id_format(cmd.kind, &cmd.model)?;
                Self::ensure_unique_generation_model(state, cmd.kind, &cmd.model, None)?;
                vec![Self::Event::GenerationModelAdded(GenerationModelAdded {
                    model_id: Uuid::new_v4(),
                    kind: cmd.kind,
                    model: cmd.model,
                })]
            }
            Self::Command::UpdateGenerationModel(cmd) => {
                let model = Self::find_generation_model(state, cmd.model_id)?;
                Self::validate_non_empty("generation model", &cmd.model)?;
                Self::validate_model_id_format(cmd.kind, &cmd.model)?;
                Self::ensure_unique_generation_model(
                    state,
                    cmd.kind,
                    &cmd.model,
                    Some(model.generation_model_id),
                )?;
                vec![Self::Event::GenerationModelUpdated(
                    GenerationModelUpdated {
                        model_id: model.generation_model_id,
                        kind: cmd.kind,
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

            Self::Command::AddVectorIndex(cmd) => {
                Self::validate_non_empty("vector index name", &cmd.name)?;
                Self::validate_positive("vector index dimensions", cmd.dimensions)?;
                Self::ensure_unique_vector_index_name(state, &cmd.name, None)?;
                vec![Self::Event::VectorIndexAdded(VectorIndexAdded {
                    index_id: Uuid::new_v4(),
                    kind: cmd.kind,
                    name: cmd.name,
                    dimensions: cmd.dimensions,
                })]
            }
            Self::Command::UpdateVectorIndex(cmd) => {
                let index = Self::find_vector_index(state, cmd.index_id)?;
                Self::validate_non_empty("vector index name", &cmd.name)?;
                Self::validate_positive("vector index dimensions", cmd.dimensions)?;
                Self::ensure_unique_vector_index_name(state, &cmd.name, Some(index.index_id))?;
                vec![Self::Event::VectorIndexUpdated(VectorIndexUpdated {
                    index_id: index.index_id,
                    kind: cmd.kind,
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

            Self::Command::CreateChunkingConfiguration(cmd) => {
                Self::validate_non_empty("chunking configuration name", &cmd.name)?;
                Self::ensure_unique_chunking_configuration_name(state, &cmd.name, None)?;
                Self::validate_chunking_config_refs(state, &cmd.config)?;
                vec![Self::Event::ChunkingConfigurationCreated(
                    ChunkingConfigurationCreated {
                        chunking_configuration_id: Uuid::new_v4(),
                        name: cmd.name,
                        config: cmd.config,
                    },
                )]
            }
            Self::Command::UpdateChunkingConfiguration(cmd) => {
                Self::find_chunking_configuration(state, cmd.chunking_configuration_id)?;
                Self::validate_non_empty("chunking configuration name", &cmd.name)?;
                Self::ensure_unique_chunking_configuration_name(
                    state,
                    &cmd.name,
                    Some(cmd.chunking_configuration_id),
                )?;
                Self::validate_chunking_config_refs(state, &cmd.config)?;
                vec![Self::Event::ChunkingConfigurationUpdated(
                    ChunkingConfigurationUpdated {
                        chunking_configuration_id: cmd.chunking_configuration_id,
                        name: cmd.name,
                        config: cmd.config,
                    },
                )]
            }
            Self::Command::DeleteChunkingConfiguration(cmd) => {
                Self::find_chunking_configuration(state, cmd.chunking_configuration_id)?;
                vec![Self::Event::ChunkingConfigurationDeleted(
                    ChunkingConfigurationDeleted {
                        chunking_configuration_id: cmd.chunking_configuration_id,
                    },
                )]
            }

            Self::Command::CreateSweepTemplate(cmd) => {
                Self::validate_non_empty("sweep template name", &cmd.name)?;
                Self::ensure_unique_sweep_template_name(state, &cmd.name, None)?;
                Self::validate_sweep_template_members(state, &cmd.members)?;
                vec![Self::Event::SweepTemplateCreated(SweepTemplateCreated {
                    sweep_template_id: Uuid::new_v4(),
                    name: cmd.name,
                    members: cmd.members,
                })]
            }
            Self::Command::UpdateSweepTemplate(cmd) => {
                Self::find_sweep_template(state, cmd.sweep_template_id)?;
                Self::validate_non_empty("sweep template name", &cmd.name)?;
                Self::ensure_unique_sweep_template_name(
                    state,
                    &cmd.name,
                    Some(cmd.sweep_template_id),
                )?;
                Self::validate_sweep_template_members(state, &cmd.members)?;
                vec![Self::Event::SweepTemplateUpdated(SweepTemplateUpdated {
                    sweep_template_id: cmd.sweep_template_id,
                    name: cmd.name,
                    members: cmd.members,
                })]
            }
            Self::Command::DeleteSweepTemplate(cmd) => {
                Self::find_sweep_template(state, cmd.sweep_template_id)?;
                vec![Self::Event::SweepTemplateDeleted(SweepTemplateDeleted {
                    sweep_template_id: cmd.sweep_template_id,
                })]
            }
            Self::Command::SetDefaultSweepTemplate(cmd) => {
                Self::find_sweep_template(state, cmd.sweep_template_id)?;
                vec![Self::Event::SweepTemplateDefaultSet(
                    SweepTemplateDefaultSet {
                        sweep_template_id: cmd.sweep_template_id,
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
            embedding_models: Vec::new(),
            generation_models: Vec::new(),
            vector_indexes: Vec::new(),
            pipeline_configurations: Vec::new(),
            chunking_configurations: Vec::new(),
            sweep_templates: Vec::new(),
            default_sweep_template_id: None,
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

    fn find_chunking_configuration(
        &self,
        id: Uuid,
    ) -> Result<&ChunkingConfiguration, ConfigurationError> {
        self.chunking_configurations
            .iter()
            .find(|cc| cc.chunking_configuration_id == id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn validate_chunking_config_refs(
        &self,
        config: &crate::shared::ChunkingConfig,
    ) -> Result<(), ConfigurationError> {
        if let crate::shared::ChunkingConfig::Llm(llm) = config {
            self.find_generation_model(llm.generation_model_id)
                .map_err(|_| {
                    ConfigurationError::ValidationError(format!(
                        "LLM chunking generation model {} not found in registry",
                        llm.generation_model_id
                    ))
                })?;
        }
        Ok(())
    }

    fn validate_model_id_format(
        kind: AiProviderKind,
        model_id: &str,
    ) -> Result<(), ConfigurationError> {
        if !kind.model_id_well_formed(model_id) {
            return Err(ConfigurationError::ValidationError(format!(
                "model id '{model_id}' is not well-formed for provider kind {}",
                kind.as_str()
            )));
        }
        Ok(())
    }

    fn find_sweep_template(&self, id: Uuid) -> Result<&SweepTemplate, ConfigurationError> {
        self.sweep_templates
            .iter()
            .find(|st| st.sweep_template_id == id)
            .ok_or(ConfigurationError::NotFound)
    }

    fn ensure_unique_sweep_template_name(
        &self,
        name: &str,
        sweep_template_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self
            .sweep_templates
            .iter()
            .any(|st| st.name == name && Some(st.sweep_template_id) != sweep_template_id)
        {
            return Err(ConfigurationError::ValidationError(format!(
                "Sweep template with name {name} already exists"
            )));
        }
        Ok(())
    }

    fn validate_sweep_template_members(&self, members: &[Uuid]) -> Result<(), ConfigurationError> {
        if members.is_empty() {
            return Err(ConfigurationError::ValidationError(
                "Sweep template must include at least one chunking configuration".into(),
            ));
        }
        for id in members {
            if !self
                .chunking_configurations
                .iter()
                .any(|cc| cc.chunking_configuration_id == *id)
            {
                return Err(ConfigurationError::ValidationError(format!(
                    "Sweep template references unknown chunking configuration {id}"
                )));
            }
        }
        Ok(())
    }

    fn ensure_unique_chunking_configuration_name(
        &self,
        name: &str,
        chunking_configuration_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self.chunking_configurations.iter().any(|cc| {
            cc.name == name && Some(cc.chunking_configuration_id) != chunking_configuration_id
        }) {
            return Err(ConfigurationError::ValidationError(format!(
                "Chunking configuration with name {name} already exists"
            )));
        }
        Ok(())
    }

    fn ensure_unique_embedding_model(
        &self,
        kind: AiProviderKind,
        model_name: &str,
        model_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self.embedding_models.iter().any(|model| {
            model.kind == kind
                && model.model == model_name
                && Some(model.embedding_model_id) != model_id
        }) {
            return Err(ConfigurationError::ValidationError(format!(
                "Embedding model {model_name} already exists for {}",
                kind.as_str()
            )));
        }
        Ok(())
    }

    fn ensure_unique_generation_model(
        &self,
        kind: AiProviderKind,
        model_name: &str,
        model_id: Option<Uuid>,
    ) -> Result<(), ConfigurationError> {
        if self.generation_models.iter().any(|model| {
            model.kind == kind
                && model.model == model_name
                && Some(model.generation_model_id) != model_id
        }) {
            return Err(ConfigurationError::ValidationError(format!(
                "Generation model {model_name} already exists for {}",
                kind.as_str()
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
        embedding_model::{AddEmbeddingModel, UpdateEmbeddingModel},
        kinds::{AiProviderKind, VectorStoreKind},
        pipeline_configuration::commands::CreatePipelineConfiguration,
    };

    use super::*;

    #[test]
    fn first_command_bootstraps_configuration() {
        let events = Configuration::handle_command(
            None,
            ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-base-en-v1.5".into(),
                dimensions: 768,
            }),
        )
        .unwrap();

        assert!(matches!(
            &events[0],
            ConfigurationEvent::ConfigurationCreated(_)
        ));
        assert!(matches!(
            &events[1],
            ConfigurationEvent::EmbeddingModelAdded(_)
        ));

        let configuration = Configuration::from_events(&events).unwrap();
        assert_eq!(
            configuration.configuration_id,
            Configuration::singleton_id()
        );
        assert_eq!(configuration.embedding_models.len(), 1);
        assert_eq!(
            configuration.embedding_models[0].kind,
            AiProviderKind::Cloudflare
        );
    }

    #[test]
    fn rejects_embedding_model_id_with_wrong_format_for_kind() {
        let error = Configuration::handle_command(
            Some(&Configuration::from_created(Configuration::singleton_id())),
            ConfigurationCommand::AddEmbeddingModel(AddEmbeddingModel {
                kind: AiProviderKind::Cloudflare,
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
            }),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigurationError::ValidationError(_)));
    }

    #[test]
    fn replay_requires_configuration_created_as_first_event() {
        let configuration = Configuration::from_events(&[ConfigurationEvent::EmbeddingModelAdded(
            EmbeddingModelAdded {
                model_id: Uuid::new_v4(),
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-base-en-v1.5".into(),
                dimensions: 768,
            },
        )]);

        assert!(configuration.is_none());
    }

    #[test]
    fn can_move_embedding_model_to_another_kind() {
        let model_id = Uuid::new_v4();
        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id,
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-base-en-v1.5".into(),
                dimensions: 768,
            }),
        ])
        .unwrap();

        let events = Configuration::handle_command(
            Some(&configuration),
            ConfigurationCommand::UpdateEmbeddingModel(UpdateEmbeddingModel {
                model_id,
                kind: AiProviderKind::Ollama,
                model: "nomic-embed-text".into(),
                dimensions: 768,
            }),
        )
        .unwrap();

        assert_eq!(
            events,
            vec![ConfigurationEvent::EmbeddingModelUpdated(
                EmbeddingModelUpdated {
                    model_id,
                    kind: AiProviderKind::Ollama,
                    model: "nomic-embed-text".into(),
                    dimensions: 768,
                }
            )]
        );
    }

    #[test]
    fn create_pipeline_configuration_validates_dimensions_match() {
        let embedding_model_id = Uuid::new_v4();
        let generation_model_id = Uuid::new_v4();
        let vector_index_id = Uuid::new_v4();

        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id: embedding_model_id,
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-large-en-v1.5".into(),
                dimensions: 1024,
            }),
            ConfigurationEvent::GenerationModelAdded(GenerationModelAdded {
                model_id: generation_model_id,
                kind: AiProviderKind::Cloudflare,
                model: "@cf/zai-org/glm-4.7-flash".into(),
            }),
            ConfigurationEvent::VectorIndexAdded(VectorIndexAdded {
                index_id: vector_index_id,
                kind: VectorStoreKind::CloudflareVectorize,
                name: "my-index".into(),
                dimensions: 768, // mismatch: embedding is 1024
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
        let embedding_model_id = Uuid::new_v4();
        let generation_model_id = Uuid::new_v4();
        let vector_index_id = Uuid::new_v4();

        let configuration = Configuration::from_events(&[
            ConfigurationEvent::ConfigurationCreated(ConfigurationCreated {
                configuration_id: Configuration::singleton_id(),
            }),
            ConfigurationEvent::EmbeddingModelAdded(EmbeddingModelAdded {
                model_id: embedding_model_id,
                kind: AiProviderKind::Cloudflare,
                model: "@cf/baai/bge-large-en-v1.5".into(),
                dimensions: 1024,
            }),
            ConfigurationEvent::GenerationModelAdded(GenerationModelAdded {
                model_id: generation_model_id,
                kind: AiProviderKind::Cloudflare,
                model: "@cf/zai-org/glm-4.7-flash".into(),
            }),
            ConfigurationEvent::VectorIndexAdded(VectorIndexAdded {
                index_id: vector_index_id,
                kind: VectorStoreKind::CloudflareVectorize,
                name: "my-index".into(),
                dimensions: 1024,
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
