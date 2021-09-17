use std::future::Future;

use crate::{Endpoint, Error, IntoResponse, Request, Result};

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
impl<E, F, Fut, R> Endpoint for MapErr<E, F>
where
    E: Endpoint<Output = Result<R>>,
    F: Fn(Error) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Error> + Send + 'static,
    R: IntoResponse,
{
    type Output = E::Output;

    async fn call(&self, req: Request) -> Self::Output {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp),
            Err(err) => Err((self.f)(err).await),
        }
    }
}
