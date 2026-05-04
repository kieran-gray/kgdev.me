use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::api_worker::application::{
    AppError, Reference,
    cache_service::{CacheError, CacheTrait},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSource {
    pub chunk_id: u32,
    pub heading: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnswer {
    pub answer: String,
    pub sources: Vec<CachedSource>,
    #[serde(default)]
    pub references: Vec<Reference>,
    pub model: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostVersion {
    v: String,
}

#[async_trait(?Send)]
pub trait QaCacheServiceTrait {
    async fn get(
        &self,
        slug: &str,
        post_version: &str,
        hash: &str,
    ) -> Result<Option<CachedAnswer>, AppError>;
    async fn put(
        &self,
        slug: &str,
        post_version: &str,
        hash: &str,
        answer: &CachedAnswer,
    ) -> Result<(), AppError>;
    async fn get_post_version(&self, slug: &str) -> Result<Option<String>, AppError>;
}

pub struct QaCacheService<C: CacheTrait + Send + Sync> {
    cache: Arc<C>,
}

impl<C: CacheTrait + Send + Sync + 'static> QaCacheService<C> {
    pub fn create(cache: Arc<C>) -> Arc<Self> {
        Arc::new(Self { cache })
    }

    fn answer_key(slug: &str, post_version: &str, hash: &str) -> String {
        format!("qa:{slug}:{post_version}:{hash}")
    }

    fn post_version_key(slug: &str) -> String {
        format!("post_version:{slug}")
    }
}

fn map_cache_err(e: CacheError) -> AppError {
    AppError::InternalError(format!("Cache error: {e}"))
}

#[async_trait(?Send)]
impl<C: CacheTrait + Send + Sync + 'static> QaCacheServiceTrait for QaCacheService<C> {
    async fn get(
        &self,
        slug: &str,
        post_version: &str,
        hash: &str,
    ) -> Result<Option<CachedAnswer>, AppError> {
        self.cache
            .get::<CachedAnswer>(Self::answer_key(slug, post_version, hash))
            .await
            .map_err(map_cache_err)
    }

    async fn put(
        &self,
        slug: &str,
        post_version: &str,
        hash: &str,
        answer: &CachedAnswer,
    ) -> Result<(), AppError> {
        self.cache
            .set(Self::answer_key(slug, post_version, hash), answer)
            .await
            .map_err(map_cache_err)
    }

    async fn get_post_version(&self, slug: &str) -> Result<Option<String>, AppError> {
        let entry = self
            .cache
            .get::<PostVersion>(Self::post_version_key(slug))
            .await
            .map_err(map_cache_err)?;
        Ok(entry.map(|e| e.v))
    }
}
