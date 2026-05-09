use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvdier {
    pub provider_id: Uuid,
    pub name: String,
}
