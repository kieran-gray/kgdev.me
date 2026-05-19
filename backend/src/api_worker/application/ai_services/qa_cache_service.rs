use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::api_worker::application::{
    AppError, Reference,
    cache_service::{CacheError, CacheTrait},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnswer {
    pub answer: String,
    #[serde(default)]
    pub references: Vec<Reference>,
    pub model: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceVersion {
    v: String,
}

#[async_trait(?Send)]
pub trait QaCacheServiceTrait {
    async fn get(
        &self,
        slug: &str,
        source_version: &str,
        hash: &str,
    ) -> Result<Option<CachedAnswer>, AppError>;
    async fn put(
        &self,
        slug: &str,
        source_version: &str,
        hash: &str,
        answer: &CachedAnswer,
    ) -> Result<(), AppError>;
    async fn get_source_version(
        &self,
        index_name: &str,
        slug: &str,
    ) -> Result<Option<String>, AppError>;
}

pub struct QaCacheService<C: CacheTrait + Send + Sync> {
    cache: Arc<C>,
}

impl<C: CacheTrait + Send + Sync + 'static> QaCacheService<C> {
    pub fn create(cache: Arc<C>) -> Arc<Self> {
        Arc::new(Self { cache })
    }

    fn answer_key(slug: &str, source_version: &str, hash: &str) -> String {
        format!("qa:{slug}:{source_version}:{hash}")
    }

    fn source_version_key(index_name: &str, slug: &str) -> String {
        format!("source_version:{index_name}:{slug}")
    }
}

fn map_cache_err(e: &CacheError) -> AppError {
    AppError::InternalError(format!("Cache error: {e}"))
}

#[async_trait(?Send)]
impl<C: CacheTrait + Send + Sync + 'static> QaCacheServiceTrait for QaCacheService<C> {
    async fn get(
        &self,
        slug: &str,
        source_version: &str,
        hash: &str,
    ) -> Result<Option<CachedAnswer>, AppError> {
        self.cache
            .get::<CachedAnswer>(Self::answer_key(slug, source_version, hash))
            .await
            .map_err(|ref e| map_cache_err(e))
    }

    async fn put(
        &self,
        slug: &str,
        source_version: &str,
        hash: &str,
        answer: &CachedAnswer,
    ) -> Result<(), AppError> {
        self.cache
            .set(Self::answer_key(slug, source_version, hash), answer)
            .await
            .map_err(|ref e| map_cache_err(e))
    }

    async fn get_source_version(
        &self,
        index_name: &str,
        slug: &str,
    ) -> Result<Option<String>, AppError> {
        let entry = self
            .cache
            .get::<SourceVersion>(Self::source_version_key(index_name, slug))
            .await
            .map_err(|ref e| map_cache_err(e))?;
        Ok(entry.map(|e| e.v))
    }
}
