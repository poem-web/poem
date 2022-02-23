use std::time::Duration;

use bytes::Bytes;

use crate::Result;

/// Represents a back-end cache storage.
#[async_trait::async_trait]
pub trait CacheStorage: Send + Sync {
    async fn set(&self, key: Bytes, value: Bytes, expires_in: Option<Duration>) -> Result<()>;

    async fn get(&self, key: Bytes) -> Result<Option<Bytes>>;

    async fn touch(&self, key: Bytes, expires_in: Option<Duration>) -> Result<()>;

    async fn delete(&self, key: Bytes) -> Result<()>;

    async fn contains_key(&self, key: Bytes) -> Result<bool>;

    async fn clear(&self) -> Result<()>;
}
