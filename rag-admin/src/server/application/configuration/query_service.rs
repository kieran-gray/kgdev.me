use std::sync::Arc;

use crate::server::application::AppError;
use crate::server::domain::pipeline_configuration::{
    PipelineConfiguration, PipelineConfigurationRepository, PipelineConfigurationRepositoryError,
};
use crate::shared::{
    AiProviderDto, EmbeddingModelDto, GenerationModelDto, PipelineConfigurationDto, VectorIndexDto,
    VectorStoreProviderDto,
};

pub struct PipelineConfigurationService {
    repository: Arc<dyn PipelineConfigurationRepository>,
}

impl PipelineConfigurationService {
    pub fn new(repository: Arc<dyn PipelineConfigurationRepository>) -> Arc<Self> {
        Arc::new(Self { repository })
    }

    pub async fn get(&self) -> Result<PipelineConfigurationDto, AppError> {
        self.repository
            .load()
            .await
            .map(map_pipeline_configuration)
            .map_err(map_repository_error)
    }
}

fn map_repository_error(error: PipelineConfigurationRepositoryError) -> AppError {
    match error {
        PipelineConfigurationRepositoryError::Internal(message) => AppError::Internal(message),
    }
}

fn map_pipeline_configuration(configuration: PipelineConfiguration) -> PipelineConfigurationDto {
    PipelineConfigurationDto {
        configuration_id: configuration.configuration_id,
        ai_providers: configuration
            .ai_providers
            .iter()
            .map(map_ai_provider)
            .collect(),
        vector_store_providers: configuration
            .vector_store_providers
            .iter()
            .map(map_vector_store_provider)
            .collect(),
        embedding_models: configuration
            .embedding_models
            .iter()
            .map(map_embedding_model)
            .collect(),
        generation_models: configuration
            .generation_models
            .iter()
            .map(map_generation_model)
            .collect(),
        vector_indexes: configuration
            .vector_indexes
            .iter()
            .map(map_vector_index)
            .collect(),
        current_embedding_model_id: configuration.current_embedding_model_id,
        current_generation_model_id: configuration.current_generation_model_id,
        current_vector_index_id: configuration.current_vector_index_id,
        current_embedding_model: configuration
            .current_embedding_model()
            .map(map_embedding_model),
        current_generation_model: configuration
            .current_generation_model()
            .map(map_generation_model),
        current_vector_index: configuration.current_vector_index().map(map_vector_index),
    }
}

fn map_ai_provider(
    provider: &crate::server::domain::ai_provider::entity::AiProvdier,
) -> AiProviderDto {
    AiProviderDto {
        provider_id: provider.provider_id,
        name: provider.name.clone(),
    }
}

fn map_embedding_model(
    model: &crate::server::domain::embedding_model::entity::EmbeddingModel,
) -> EmbeddingModelDto {
    EmbeddingModelDto {
        embedding_model_id: model.embedding_model_id,
        provider_id: model.provider_id,
        model: model.model.clone(),
        dimensions: model.dimensions,
    }
}

fn map_generation_model(
    model: &crate::server::domain::generation_model::entity::GenerationModel,
) -> GenerationModelDto {
    GenerationModelDto {
        generation_model_id: model.generation_model_id,
        provider_id: model.provider_id,
        model: model.model.clone(),
    }
}

fn map_vector_store_provider(
    provider: &crate::server::domain::vector_store_provider::entity::VectorStoreProvider,
) -> VectorStoreProviderDto {
    VectorStoreProviderDto {
        provider_id: provider.provider_id,
        name: provider.name.clone(),
    }
}

fn map_vector_index(
    index: &crate::server::domain::vector_index::entity::VectorIndex,
) -> VectorIndexDto {
    VectorIndexDto {
        index_id: index.index_id,
        vector_store_provider_id: index.vector_store_provider_id,
        name: index.name.clone(),
        dimensions: index.dimensions,
    }
}
