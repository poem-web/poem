use std::time::Duration;

use bytes::Bytes;

/// Represents a back-end cache storage.
#[async_trait::async_trait]
pub trait CacheStorage: Send + Sync {
    async fn set(&self, key: &[u8], value: &[u8], timeout: Option<Duration>, version: u64);

    async fn get(&self, key: &[u8], version: u64) -> Option<Bytes>;

    async fn touch(&self, key: &[u8], version: i64);

    async fn delete(&self, key: &[u8], version: i64);

    async fn contains_key(&self, key: &[u8], version: i64) -> bool;

    async fn clear(&self, key: &[u8]);
}
