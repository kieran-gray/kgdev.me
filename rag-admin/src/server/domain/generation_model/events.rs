use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GenerationModelAdded {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GenerationModelUpdated {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GenerationModelRemoved {
    pub model_id: Uuid,
}
