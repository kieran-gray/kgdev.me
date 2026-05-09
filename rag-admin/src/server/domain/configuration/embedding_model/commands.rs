use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddEmbeddingModel {
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateEmbeddingModel {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveEmbeddingModel {
    pub model_id: Uuid,
}
