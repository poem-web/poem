use std::future::Future;

use crate::{Endpoint, Request, Response, Result};

/// Endpoint for the [`map_request`](super::EndpointExt::map_request) method.
pub struct MapRequest<E, F> {
    inner: E,
    f: F,
}

impl<E, F> MapRequest<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> MapRequest<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut> Endpoint for MapRequest<E, F>
where
    E: Endpoint,
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Request>> + Send + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self.inner.call((self.f)(req).await?).await
    }
}
