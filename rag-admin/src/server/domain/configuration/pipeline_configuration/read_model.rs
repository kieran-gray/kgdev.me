use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    server::domain::configuration::ConfigurationReadModel, shared::PipelineConfigurationDto,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfigurationReadModel {
    pub pipeline_configuration_id: Uuid,
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

impl From<(&PipelineConfigurationReadModel, &ConfigurationReadModel)> for PipelineConfigurationDto {
    fn from(
        (pipeline_config, config): (&PipelineConfigurationReadModel, &ConfigurationReadModel),
    ) -> Self {
        Self {
            pipeline_configuration_id: pipeline_config.pipeline_configuration_id,
            name: pipeline_config.name.clone(),
            embedding_model_id: pipeline_config.embedding_model_id,
            embedding_model_name: config
                .embedding_models
                .iter()
                .find(|m| m.embedding_model_id == pipeline_config.embedding_model_id)
                .map(|m| m.model.clone()),
            generation_model_id: pipeline_config.generation_model_id,
            generation_model_name: config
                .generation_models
                .iter()
                .find(|m| m.generation_model_id == pipeline_config.generation_model_id)
                .map(|m| m.model.clone()),
            vector_index_id: pipeline_config.vector_index_id,
            vector_index_name: config
                .vector_indexes
                .iter()
                .find(|i| i.index_id == pipeline_config.vector_index_id)
                .map(|i| i.name.clone()),
        }
    }
}
