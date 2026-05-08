use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModel {
    pub embedding_model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
    pub dimensions: u32,
}
