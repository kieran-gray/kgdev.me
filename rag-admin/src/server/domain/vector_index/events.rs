use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexAdded {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexUpdated {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexRemoved {
    pub index_id: Uuid,
}
