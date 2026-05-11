use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::ChunkingConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkingConfigurationCreated {
    pub chunking_configuration_id: Uuid,
    pub name: String,
    pub config: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkingConfigurationUpdated {
    pub chunking_configuration_id: Uuid,
    pub name: String,
    pub config: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkingConfigurationDeleted {
    pub chunking_configuration_id: Uuid,
}
