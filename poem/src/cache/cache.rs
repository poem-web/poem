use std::{marker::PhantomData, sync::Arc, time::Duration};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{cache::CacheStorage, FromRequest, Request, RequestBody, Result};

#[derive(Debug, Default)]
pub struct CacheSetOptions {
    timeout: Option<Duration>,
    version: Option<u64>,
}

impl CacheSetOptions {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn timeout(self, timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            ..self
        }
    }

    #[must_use]
    pub fn version(self, version: u64) -> Self {
        Self {
            version: Some(version),
            ..self
        }
    }
}

#[derive(Debug, Default)]
pub struct CacheGetOptions {
    version: Option<u64>,
}

impl CacheGetOptions {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn version(self, version: u64) -> Self {
        Self {
            version: Some(version),
            ..self
        }
    }
}

pub struct Cache<K, V> {
    storage: Arc<dyn CacheStorage>,
    _mark1: PhantomData<K>,
    _mark2: PhantomData<V>,
}

impl<K, V> Cache<K, V>
where
    K: Serialize,
    V: Serialize + DeserializeOwned,
{
    pub fn set(
        &self,
        key: &K,
        value: &V,
        options: impl Into<Option<CacheSetOptions>>,
    ) -> Result<()> {
        todo!()
    }

    pub fn get(&self, key: &K, options: impl Into<Option<CacheGetOptions>>) -> Result<Option<V>> {
        todo!()
    }

    pub async fn touch(&self, key: &K, options: impl Into<Option<CacheGetOptions>>) -> Result<()> {
        todo!()
    }

    pub async fn delete(&self, key: &K, options: impl Into<Option<CacheGetOptions>>) -> Result<()> {
        todo!()
    }

    pub async fn contains_key(&self, key: &K, options: impl Into<Option<CacheGetOptions>>) -> bool {
        todo!()
    }

    pub fn clear(&self, key: &K, options: impl Into<Option<CacheGetOptions>>) -> Result<()> {
        todo!()
    }
}

#[async_trait::async_trait]
impl<'a, K, V> FromRequest<'a> for Cache<K, V> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        todo!()
    }
}
