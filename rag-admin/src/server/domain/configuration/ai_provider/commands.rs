use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddAiProvider {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateAiProvider {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveAiProvider {
    pub provider_id: Uuid,
}
