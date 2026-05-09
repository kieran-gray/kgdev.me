use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddVectorStoreProvider {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateVectorStoreProvider {
    pub provider_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveVectorStoreProvider {
    pub provider_id: Uuid,
}
