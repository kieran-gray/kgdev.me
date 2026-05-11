use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::kinds::VectorStoreKind;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddVectorIndex {
    pub kind: VectorStoreKind,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateVectorIndex {
    pub index_id: Uuid,
    pub kind: VectorStoreKind,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoveVectorIndex {
    pub index_id: Uuid,
}
