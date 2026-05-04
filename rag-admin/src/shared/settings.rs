use serde::{Deserialize, Serialize};

use super::chunking::ChunkingConfig;
use super::embedding::EmbeddingModel;
use super::vector::VectorIndexConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SettingsDto {
    pub blog_url: String,
    pub cloudflare_account_id: String,
    pub cloudflare_api_token: String,
    pub kv_namespace_id: String,
    #[serde(default)]
    pub vector_index: VectorIndexConfig,
    #[serde(default)]
    pub embedding_model: EmbeddingModel,
    #[serde(default)]
    pub default_chunking: ChunkingConfig,
}
