use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::server::application::AppError;
use crate::shared::ChunkingConfig;

#[async_trait]
pub trait PostChunkingConfigStore: Send + Sync {
    async fn all(&self) -> Result<BTreeMap<String, ChunkingConfig>, AppError>;
    async fn get(&self, slug: &str) -> Result<Option<ChunkingConfig>, AppError>;
    async fn save(&self, slug: &str, config: ChunkingConfig) -> Result<(), AppError>;
    async fn clear(&self, slug: &str) -> Result<(), AppError>;
}
