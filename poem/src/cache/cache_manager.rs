use crate::cache::{Cache, CacheStorage};
use crate::{Endpoint, IntoResponse, Middleware, Request, Response};
use std::sync::Arc;

pub struct CacheManager {
    version: u64,
    storage: Arc<dyn CacheStorage>,
}

impl CacheManager {
    pub fn new(storage: impl CacheStorage + 'static) -> Self {
        Self {
            version: 1,
            storage: Arc::new(storage),
        }
    }

    pub fn version(self, version: u64) -> Self {
        Self { version, ..self }
    }
}

impl<E: Endpoint> Middleware<E> for CacheManager {
    type Output = CacheManagerEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CacheManagerEndpoint {
            inner: ep,
            version: self.version,
            storage: self.storage.clone(),
        }
    }
}

pub struct CacheManagerEndpoint<E> {
    inner: E,
    version: u64,
    storage: Arc<dyn CacheStorage>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CacheManagerEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> crate::Result<Self::Output> {
        req.extensions_mut()
            .insert(Cache::new(self.storage.clone(), self.version));
        let resp = self.inner.call(req).await?.into_response();
        Ok(resp)
    }
}
