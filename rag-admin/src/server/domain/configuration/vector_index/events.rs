use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::domain::configuration::kinds::VectorStoreKind;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexAdded {
    pub index_id: Uuid,
    pub kind: VectorStoreKind,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexUpdated {
    pub index_id: Uuid,
    pub kind: VectorStoreKind,
    pub name: String,
    pub dimensions: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VectorIndexRemoved {
    pub index_id: Uuid,
}
