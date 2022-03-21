use bytes::Bytes;
use std::{sync::Arc, time::Duration};

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

pub struct Cache {
    storage: Arc<dyn CacheStorage>,
    version: u64,
}

impl Cache {
    #[inline]
    pub(crate) fn new(storage: Arc<dyn CacheStorage>, version: u64) -> Self {
        Self { storage, version }
    }

    pub async fn set(
        &self,
        key: &str,
        value: impl Into<Bytes>,
        options: impl Into<Option<CacheSetOptions>>,
    ) -> Result<()> {
        let options = options.into().unwrap_or_default();
        let timeout = options.timeout;
        let version = options.version.unwrap_or(self.version);
        self.storage.set(version, key, value.into(), timeout).await
    }

    pub async fn get(
        &self,
        key: &str,
        options: impl Into<Option<CacheGetOptions>>,
    ) -> Result<Option<Bytes>> {
        let options = options.into().unwrap_or_default();
        let version = options.version.unwrap_or(self.version);
        self.storage.get(version, key).await
    }

    pub async fn touch(
        &self,
        key: &str,
        options: impl Into<Option<CacheSetOptions>>,
    ) -> Result<()> {
        let options = options.into().unwrap_or_default();
        let timeout = options.timeout;
        let version = options.version.unwrap_or(self.version);
        self.storage.touch(version, key, timeout).await
    }

    pub async fn delete(
        &self,
        key: &str,
        options: impl Into<Option<CacheGetOptions>>,
    ) -> Result<()> {
        let options = options.into().unwrap_or_default();
        let version = options.version.unwrap_or(self.version);
        self.storage.delete(version, key).await
    }

    pub async fn contains_key(
        &self,
        key: &str,
        options: impl Into<Option<CacheGetOptions>>,
    ) -> Result<bool> {
        let options = options.into().unwrap_or_default();
        let version = options.version.unwrap_or(self.version);
        self.storage.contains_key(version, key).await
    }

    pub async fn clear(&self, options: impl Into<Option<CacheGetOptions>>) -> Result<()> {
        let options = options.into().unwrap_or_default();
        let version = options.version.unwrap_or(self.version);
        self.storage.clear(version).await
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a Cache {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<Cache>()
            .expect("To use the `Cache` extractor, the `CacheManager` middleware is required."))
    }
}
