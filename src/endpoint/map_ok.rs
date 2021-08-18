use std::future::Future;

use crate::{Endpoint, Request, Response, Result};

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
impl<E, F, Fut> Endpoint for MapOk<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => Ok((self.f)(resp).await),
            Err(err) => Err(err),
        }
    }
}
