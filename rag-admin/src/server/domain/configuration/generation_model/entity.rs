use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationModel {
    pub generation_model_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
}
