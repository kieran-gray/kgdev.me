use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum CacheError {
    WriteError(String),
    ReadError(String),
    DeleteError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::WriteError(msg) => write!(f, "Write error: {msg}"),
            CacheError::ReadError(msg) => write!(f, "Read error: {msg}"),
            CacheError::DeleteError(msg) => write!(f, "Delete error: {msg}"),
        }
    }
}

impl std::error::Error for CacheError {}

#[async_trait(?Send)]
pub trait CacheTrait {
    async fn set<T: Serialize>(&self, key: String, value: T) -> Result<(), CacheError>;
    async fn set_with_ttl<T: Serialize>(
        &self,
        key: String,
        value: T,
        ttl_seconds: u64,
    ) -> Result<(), CacheError>;
    async fn get<T: for<'de> Deserialize<'de>>(&self, key: String)
    -> Result<Option<T>, CacheError>;
    async fn clear(&self, key: String) -> Result<(), CacheError>;
}
