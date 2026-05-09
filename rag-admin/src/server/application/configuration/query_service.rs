use std::sync::Arc;

use crate::server::application::AppError;
use crate::server::domain::configuration::{
    ConfigurationReadModel, ConfigurationRepository, ConfigurationRepositoryError,
};
use crate::server::domain::pipeline_configuration::{
    PipelineConfigurationRepository, PipelineConfigurationRepositoryError,
};
use crate::shared::{
    AiProviderDto, ConfigurationDto, EmbeddingModelDto, GenerationModelDto,
    PipelineConfigurationDto, VectorIndexDto, VectorStoreProviderDto,
};

pub struct ConfigurationQueryService {
    repository: Arc<dyn ConfigurationRepository>,
}

impl ConfigurationQueryService {
    pub fn new(repository: Arc<dyn ConfigurationRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn get(&self) -> Result<ConfigurationDto, AppError> {
        self.repository
            .load()
            .await
            .map(map_configuration)
            .map_err(|e| match e {
                ConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })
    }
}

pub struct PipelineConfigurationQueryService {
    pipeline_repository: Arc<dyn PipelineConfigurationRepository>,
    configuration_repository: Arc<dyn ConfigurationRepository>,
}

impl PipelineConfigurationQueryService {
    pub fn new(
        pipeline_repository: Arc<dyn PipelineConfigurationRepository>,
        configuration_repository: Arc<dyn ConfigurationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pipeline_repository,
            configuration_repository,
        })
    }

    pub async fn list(&self) -> Result<Vec<PipelineConfigurationDto>, AppError> {
        let pipeline_configs = self
            .pipeline_repository
            .load_all()
            .await
            .map_err(|e| match e {
                PipelineConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })?;

        let catalog = self
            .configuration_repository
            .load()
            .await
            .map_err(|e| match e {
                ConfigurationRepositoryError::Internal(m) => AppError::Internal(m),
            })?;

        Ok(pipeline_configs
            .iter()
            .map(|pc| map_pipeline_configuration(pc, &catalog))
            .collect())
    }
}

fn map_configuration(read_model: ConfigurationReadModel) -> ConfigurationDto {
    ConfigurationDto {
        configuration_id: read_model.configuration_id,
        ai_providers: read_model
            .ai_providers
            .iter()
            .map(|p| AiProviderDto {
                provider_id: p.provider_id,
                name: p.name.clone(),
            })
            .collect(),
        vector_store_providers: read_model
            .vector_store_providers
            .iter()
            .map(|p| VectorStoreProviderDto {
                provider_id: p.provider_id,
                name: p.name.clone(),
            })
            .collect(),
        embedding_models: read_model
            .embedding_models
            .iter()
            .map(|m| EmbeddingModelDto {
                embedding_model_id: m.embedding_model_id,
                provider_id: m.provider_id,
                model: m.model.clone(),
                dimensions: m.dimensions,
            })
            .collect(),
        generation_models: read_model
            .generation_models
            .iter()
            .map(|m| GenerationModelDto {
                generation_model_id: m.generation_model_id,
                provider_id: m.provider_id,
                model: m.model.clone(),
            })
            .collect(),
        vector_indexes: read_model
            .vector_indexes
            .iter()
            .map(|i| VectorIndexDto {
                index_id: i.index_id,
                vector_store_provider_id: i.vector_store_provider_id,
                name: i.name.clone(),
                dimensions: i.dimensions,
            })
            .collect(),
    }
}

fn map_pipeline_configuration(
    pc: &crate::server::domain::pipeline_configuration::PipelineConfigurationReadModel,
    catalog: &ConfigurationReadModel,
) -> PipelineConfigurationDto {
    let embedding_model_name = catalog
        .embedding_models
        .iter()
        .find(|m| m.embedding_model_id == pc.embedding_model_id)
        .map(|m| m.model.clone());

    let generation_model_name = catalog
        .generation_models
        .iter()
        .find(|m| m.generation_model_id == pc.generation_model_id)
        .map(|m| m.model.clone());

    let vector_index_name = catalog
        .vector_indexes
        .iter()
        .find(|i| i.index_id == pc.vector_index_id)
        .map(|i| i.name.clone());

    PipelineConfigurationDto {
        pipeline_configuration_id: pc.pipeline_configuration_id,
        name: pc.name.clone(),
        embedding_model_id: pc.embedding_model_id,
        embedding_model_name,
        generation_model_id: pc.generation_model_id,
        generation_model_name,
        vector_index_id: pc.vector_index_id,
        vector_index_name,
    }
}
