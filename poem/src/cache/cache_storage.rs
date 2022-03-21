use std::time::Duration;

use bytes::Bytes;

use crate::Result;

/// Represents a back-end cache storage.
#[async_trait::async_trait]
pub trait CacheStorage: Send + Sync {
    async fn set(
        &self,
        version: u64,
        key: &str,
        value: Bytes,
        expires_in: Option<Duration>,
    ) -> Result<()>;

    async fn get(&self, version: u64, key: &str) -> Result<Option<Bytes>>;

    async fn touch(&self, version: u64, key: &str, expires_in: Option<Duration>) -> Result<()>;

    async fn delete(&self, version: u64, key: &str) -> Result<()>;

    async fn contains_key(&self, version: u64, key: &str) -> Result<bool>;

    async fn clear(&self, version: u64) -> Result<()>;
}
