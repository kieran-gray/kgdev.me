use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api_worker::application::{
    AppError,
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
    pub model: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DailyCounter {
    date: String,
    count: u32,
}

#[async_trait(?Send)]
pub trait QaCacheServiceTrait {
    async fn get(&self, slug: &str, hash: &str) -> Result<Option<CachedAnswer>, AppError>;
    async fn put(&self, slug: &str, hash: &str, answer: &CachedAnswer) -> Result<(), AppError>;
    async fn check_and_increment_daily_cap(&self, cap: u32) -> Result<bool, AppError>;
}

pub struct QaCacheService<C: CacheTrait + Send + Sync> {
    cache: Arc<C>,
}

impl<C: CacheTrait + Send + Sync + 'static> QaCacheService<C> {
    pub fn create(cache: Arc<C>) -> Arc<Self> {
        Arc::new(Self { cache })
    }

    fn answer_key(slug: &str, hash: &str) -> String {
        format!("qa:{slug}:{hash}")
    }

    fn daily_key() -> String {
        "cap:daily".to_string()
    }
}

fn map_cache_err(e: CacheError) -> AppError {
    AppError::InternalError(format!("Cache error: {e}"))
}

#[async_trait(?Send)]
impl<C: CacheTrait + Send + Sync + 'static> QaCacheServiceTrait for QaCacheService<C> {
    async fn get(&self, slug: &str, hash: &str) -> Result<Option<CachedAnswer>, AppError> {
        self.cache
            .get::<CachedAnswer>(Self::answer_key(slug, hash))
            .await
            .map_err(map_cache_err)
    }

    async fn put(&self, slug: &str, hash: &str, answer: &CachedAnswer) -> Result<(), AppError> {
        self.cache
            .set(Self::answer_key(slug, hash), answer)
            .await
            .map_err(map_cache_err)
    }

    async fn check_and_increment_daily_cap(&self, cap: u32) -> Result<bool, AppError> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let key = Self::daily_key();

        let current = self
            .cache
            .get::<DailyCounter>(key.clone())
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "daily counter read failed; treating as fresh day");
                None
            });

        let next = match current {
            Some(c) if c.date == today => {
                if c.count >= cap {
                    return Ok(false);
                }
                DailyCounter {
                    date: today,
                    count: c.count + 1,
                }
            }
            _ => DailyCounter {
                date: today,
                count: 1,
            },
        };

        if let Err(e) = self.cache.set(key, &next).await {
            warn!(error = %e, "daily counter write failed; allowing request");
        }
        Ok(true)
    }
}
