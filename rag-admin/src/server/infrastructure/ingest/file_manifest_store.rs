use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::Mutex;

use crate::server::application::ingest::ports::ManifestStore;
use crate::server::application::AppError;
use crate::server::domain::{Manifest, ManifestEntry};

pub struct FileManifestStore {
    path: PathBuf,
    cache: Mutex<Option<Manifest>>,
}

impl FileManifestStore {
    pub fn new(path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            path,
            cache: Mutex::new(None),
        })
    }

    async fn read_from_disk(&self) -> Result<Manifest, AppError> {
        if !self.path.exists() {
            return Ok(Manifest::default());
        }
        let bytes = fs::read(&self.path)
            .await
            .map_err(|e| AppError::Io(format!("read manifest: {e}")))?;
        if bytes.is_empty() {
            return Ok(Manifest::default());
        }
        serde_json::from_slice(&bytes).map_err(|e| AppError::Io(format!("parse manifest: {e}")))
    }

    async fn write_to_disk(&self, manifest: &Manifest) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Io(format!("create manifest dir: {e}")))?;
        }
        let mut s = serde_json::to_string_pretty(manifest)
            .map_err(|e| AppError::Internal(format!("encode manifest: {e}")))?;
        s.push('\n');
        fs::write(&self.path, s)
            .await
            .map_err(|e| AppError::Io(format!("write manifest: {e}")))
    }
}

#[async_trait]
impl ManifestStore for FileManifestStore {
    async fn load(&self) -> Result<Manifest, AppError> {
        let mut cache = self.cache.lock().await;
        if let Some(m) = cache.as_ref() {
            return Ok(m.clone());
        }
        let manifest = self.read_from_disk().await?;
        *cache = Some(manifest.clone());
        Ok(manifest)
    }

    async fn record(&self, slug: &str, entry: ManifestEntry) -> Result<(), AppError> {
        let mut cache = self.cache.lock().await;
        if cache.is_none() {
            *cache = Some(self.read_from_disk().await?);
        }
        let manifest = cache.as_mut().unwrap();
        manifest.posts.insert(slug.to_string(), entry);
        self.write_to_disk(manifest).await
    }
}
