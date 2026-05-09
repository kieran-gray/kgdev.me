use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddVectorIndex {
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateVectorIndex {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveVectorIndex {
    pub index_id: Uuid,
}
