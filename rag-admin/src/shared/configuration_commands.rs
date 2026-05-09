use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderType {
    Ai,
    VectorStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProviderDto {
    pub name: String,
    pub provider_type: ProviderType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveAiProviderDto {
    pub provider_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAiProviderDto {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEmbeddingModelDto {
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEmbeddingModelDto {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveEmbeddingModelDto {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGenerationModelDto {
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGenerationModelDto {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveGenerationModelDto {
    pub model_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVectorStoreProviderDto {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveVectorStoreProviderDto {
    pub provider_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVectorIndexDto {
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVectorIndexDto {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveVectorIndexDto {
    pub index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineConfigurationDto {
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePipelineConfigurationDto {
    pub pipeline_configuration_id: Uuid,
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePipelineConfigurationDto {
    pub pipeline_configuration_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConfigurationCommandDto {
    AddProvider(AddProviderDto),
    UpdateAiProvider(UpdateAiProviderDto),
    RemoveAiProvider(RemoveAiProviderDto),
    AddEmbeddingModel(AddEmbeddingModelDto),
    UpdateEmbeddingModel(UpdateEmbeddingModelDto),
    RemoveEmbeddingModel(RemoveEmbeddingModelDto),
    AddGenerationModel(AddGenerationModelDto),
    UpdateGenerationModel(UpdateGenerationModelDto),
    RemoveGenerationModel(RemoveGenerationModelDto),
    UpdateVectorStoreProvider(UpdateVectorStoreProviderDto),
    RemoveVectorStoreProvider(RemoveVectorStoreProviderDto),
    AddVectorIndex(AddVectorIndexDto),
    UpdateVectorIndex(UpdateVectorIndexDto),
    RemoveVectorIndex(RemoveVectorIndexDto),
    CreatePipelineConfiguration(CreatePipelineConfigurationDto),
    UpdatePipelineConfiguration(UpdatePipelineConfigurationDto),
    DeletePipelineConfiguration(DeletePipelineConfigurationDto),
}
