use std::time::Duration;

use bytes::Bytes;
use lru::LruCache;

use crate::cache::CacheStorage;

pub struct MemoryStorage {
    lru: LruCache<Bytes, Bytes>,
}

#[async_trait::async_trait]
impl CacheStorage for MemoryStorage {
    async fn set(&self, key: &[u8], value: &[u8], timeout: Option<Duration>, version: u64) {
        todo!()
    }

    async fn get(&self, key: &[u8], version: u64) -> Option<Bytes> {
        todo!()
    }

    async fn touch(&self, key: &[u8], version: i64) {
        todo!()
    }

    async fn delete(&self, key: &[u8], version: i64) {
        todo!()
    }

    async fn contains_key(&self, key: &[u8], version: i64) -> bool {
        todo!()
    }

    async fn clear(&self, key: &[u8]) {
        todo!()
    }
}
