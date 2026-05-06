use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::server::application::blog::ports::PostChunkingConfigStore;
use crate::server::application::AppError;
use crate::shared::ChunkingConfig;

pub struct FilePostChunkingConfigStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FilePostChunkingConfigStore {
    pub fn new(path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            path,
            lock: Mutex::new(()),
        })
    }

    async fn load_unlocked(&self) -> Result<PostChunkingConfigFile, AppError> {
        if !self.path.exists() {
            return Ok(PostChunkingConfigFile::default());
        }
        let bytes = tokio::fs::read(&self.path)
            .await
            .map_err(|e| AppError::Io(format!("read post chunking config: {e}")))?;
        if bytes.is_empty() {
            return Ok(PostChunkingConfigFile::default());
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Internal(format!("parse post chunking config: {e}")))
    }

    async fn save_unlocked(&self, data: &PostChunkingConfigFile) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create post chunking config dir: {e}")))?;
        }
        let bytes = serde_json::to_vec_pretty(data)
            .map_err(|e| AppError::Internal(format!("encode post chunking config: {e}")))?;
        tokio::fs::write(&self.path, bytes)
            .await
            .map_err(|e| AppError::Io(format!("write post chunking config: {e}")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostChunkingConfigFile {
    version: u32,
    posts: BTreeMap<String, ChunkingConfig>,
}

impl Default for PostChunkingConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            posts: BTreeMap::new(),
        }
    }
}

#[async_trait]
impl PostChunkingConfigStore for FilePostChunkingConfigStore {
    async fn all(&self) -> Result<BTreeMap<String, ChunkingConfig>, AppError> {
        let _guard = self.lock.lock().await;
        Ok(self.load_unlocked().await?.posts)
    }

    async fn get(&self, slug: &str) -> Result<Option<ChunkingConfig>, AppError> {
        let _guard = self.lock.lock().await;
        Ok(self.load_unlocked().await?.posts.get(slug).copied())
    }

    async fn save(&self, slug: &str, config: ChunkingConfig) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let mut data = self.load_unlocked().await?;
        data.posts.insert(slug.to_string(), config);
        self.save_unlocked(&data).await
    }

    async fn clear(&self, slug: &str) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let mut data = self.load_unlocked().await?;
        data.posts.remove(slug);
        self.save_unlocked(&data).await
    }
}
