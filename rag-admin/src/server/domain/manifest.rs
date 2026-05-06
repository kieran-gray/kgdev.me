use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::shared::{ChunkingConfig, EmbeddingModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub posts: BTreeMap<String, ManifestEntry>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: 1,
            posts: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub post_version: String,
    pub chunk_count: u32,
    pub ingested_at: String,
    #[serde(default)]
    pub chunking_config: Option<ChunkingConfig>,
    #[serde(default)]
    pub embedding_model: Option<EmbeddingModel>,
}
