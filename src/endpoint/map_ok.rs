use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`map_ok`](super::EndpointExt::map_ok) method.
pub struct MapOk<E, F> {
    inner: E,
    f: F,
}

impl<E, F> MapOk<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> MapOk<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R> Endpoint for MapOk<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => (self.f)(resp).await.into_response(),
            Err(err) => Err(err),
        }
    }
}
