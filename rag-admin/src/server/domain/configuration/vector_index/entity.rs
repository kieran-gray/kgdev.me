use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndex {
    pub index_id: Uuid,
    pub vector_store_provider_id: Uuid,
    pub name: String,
    pub dimensions: u32,
}
