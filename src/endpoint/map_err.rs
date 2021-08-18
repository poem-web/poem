use std::future::Future;

use crate::{Endpoint, Error, Request, Response, Result};

/// Endpoint for the [`map_err`](super::EndpointExt::map_err) method.
pub struct MapErr<E, F> {
    inner: E,
    f: F,
}

impl<E, F> MapErr<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> MapErr<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut> Endpoint for MapErr<E, F>
where
    E: Endpoint,
    F: Fn(Error) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Error> + Send + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp),
            Err(err) => Err((self.f)(err).await),
        }
    }
}
