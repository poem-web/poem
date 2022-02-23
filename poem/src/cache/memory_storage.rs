use std::time::{Duration, Instant};

use bytes::Bytes;
use lru::LruCache;
use parking_lot::Mutex;

use crate::{cache::CacheStorage, Result};

struct Entry {
    value: Bytes,
    expired_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self) -> bool {
        let now = Instant::now();
        match &self.expired_at {
            Some(expired_at) => &now > expired_at,
            None => false,
        }
    }
}

pub struct MemoryStorage {
    lru: Mutex<LruCache<Bytes, Entry>>,
}

#[async_trait::async_trait]
impl CacheStorage for MemoryStorage {
    async fn set(&self, key: Bytes, value: Bytes, expires_in: Option<Duration>) -> Result<()> {
        self.lru.lock().put(
            key.into(),
            Entry {
                value: value.into(),
                expired_at: expires_in.map(|timeout| Instant::now() + timeout),
            },
        );
        Ok(())
    }

    async fn get(&self, key: Bytes) -> Result<Option<Bytes>> {
        Ok(self
            .lru
            .lock()
            .get(&key)
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.value.clone()))
    }

    async fn touch(&self, key: Bytes, expires_in: Option<Duration>) -> Result<()> {
        if let Some(entry) = self.lru.lock().get_mut(&key) {
            entry.expired_at = expires_in.map(|timeout| Instant::now() + timeout);
        }
        Ok(())
    }

    async fn delete(&self, key: Bytes) -> Result<()> {
        self.lru.lock().pop(&key);
        Ok(())
    }

    async fn contains_key(&self, key: Bytes) -> Result<bool> {
        Ok(self.lru.lock().contains(&key))
    }

    async fn clear(&self) -> Result<()> {
        self.lru.lock().clear();
        Ok(())
    }
}
