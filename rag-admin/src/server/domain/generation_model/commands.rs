use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddGenerationModel {
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateGenerationModel {
    pub model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveGenerationModel {
    pub model_id: Uuid,
}
