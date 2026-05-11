use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::kinds::AiProviderKind;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EmbeddingModelAdded {
    pub model_id: Uuid,
    pub kind: AiProviderKind,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EmbeddingModelUpdated {
    pub model_id: Uuid,
    pub kind: AiProviderKind,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EmbeddingModelRemoved {
    pub model_id: Uuid,
}
