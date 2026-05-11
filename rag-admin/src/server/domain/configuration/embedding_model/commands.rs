use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::kinds::AiProviderKind;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddEmbeddingModel {
    pub kind: AiProviderKind,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateEmbeddingModel {
    pub model_id: Uuid,
    pub kind: AiProviderKind,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveEmbeddingModel {
    pub model_id: Uuid,
}
