use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiProviderDto {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingModelDto {
    pub embedding_model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationModelDto {
    pub generation_model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorStoreProviderDto {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorIndexDto {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineConfigurationDto {
    pub configuration_id: Uuid,
    pub ai_providers: Vec<AiProviderDto>,
    pub vector_store_providers: Vec<VectorStoreProviderDto>,
    pub embedding_models: Vec<EmbeddingModelDto>,
    pub generation_models: Vec<GenerationModelDto>,
    pub vector_indexes: Vec<VectorIndexDto>,
    pub current_embedding_model_id: Option<Uuid>,
    pub current_generation_model_id: Option<Uuid>,
    pub current_vector_index_id: Option<Uuid>,
    pub current_embedding_model: Option<EmbeddingModelDto>,
    pub current_generation_model: Option<GenerationModelDto>,
    pub current_vector_index: Option<VectorIndexDto>,
}
