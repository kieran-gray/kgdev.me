use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::kinds::AiProviderKind;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddGenerationModel {
    pub kind: AiProviderKind,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateGenerationModel {
    pub model_id: Uuid,
    pub kind: AiProviderKind,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveGenerationModel {
    pub model_id: Uuid,
}
