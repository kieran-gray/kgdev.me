use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;
use worker::kv::KvStore;

use crate::api_worker::application::cache_service::{CacheError, CacheTrait};

pub struct KVCache {
    cache: KvStore,
    ttl: u64,
}

impl KVCache {
    pub fn create(cache: KvStore, ttl: u64) -> Self {
        Self { cache, ttl }
    }
}

#[async_trait(?Send)]
impl CacheTrait for KVCache {
    async fn set<T: Serialize>(&self, key: String, value: T) -> Result<(), CacheError> {
        self.cache
            .put(&key, value)
            .unwrap()
            .expiration_ttl(self.ttl)
            .execute()
            .await
            .map_err(|_| {
                let err = "Failed to write to cache".to_string();
                warn!(cache_key = %key, err);
                CacheError::WriteError(err)
            })
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, id: String) -> Result<Option<T>, CacheError> {
        let cache_result = self.cache.get(&id).json::<T>().await.map_err(|_| {
            let err = "Failed to fetch from cache".to_string();
            warn!(cache_key = %id, err);
            CacheError::ReadError(err)
        })?;
        Ok(cache_result)
    }

    async fn clear(&self, id: String) -> Result<(), CacheError> {
        self.cache.delete(&id).await.map_err(|_| {
            let err = "Failed to delete from cache".to_string();
            warn!(cache_key = %id, err);
            CacheError::DeleteError(err)
        })?;
        Ok(())
    }
}
