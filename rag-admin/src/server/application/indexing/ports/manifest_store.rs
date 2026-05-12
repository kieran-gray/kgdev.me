use async_trait::async_trait;

use crate::server::application::AppError;
use crate::server::domain::{Manifest, ManifestEntry};

#[async_trait]
pub trait ManifestStore: Send + Sync {
    async fn load(&self) -> Result<Manifest, AppError>;
    async fn record(&self, slug: &str, entry: ManifestEntry) -> Result<(), AppError>;
}
