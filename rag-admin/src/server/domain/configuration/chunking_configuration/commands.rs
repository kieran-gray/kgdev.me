use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::ChunkingConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChunkingConfiguration {
    pub name: String,
    pub config: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChunkingConfiguration {
    pub chunking_configuration_id: Uuid,
    pub name: String,
    pub config: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteChunkingConfiguration {
    pub chunking_configuration_id: Uuid,
}
