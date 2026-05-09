use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineConfiguration {
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePipelineConfiguration {
    pub pipeline_configuration_id: Uuid,
    pub name: String,
    pub embedding_model_id: Uuid,
    pub generation_model_id: Uuid,
    pub vector_index_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePipelineConfiguration {
    pub pipeline_configuration_id: Uuid,
}