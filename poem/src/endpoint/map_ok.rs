use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Result};

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
impl<E, F, Fut, R, R2> Endpoint for MapOk<E, F>
where
    E: Endpoint<Output = Result<R>>,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = R2> + Send,
    R: IntoResponse,
    R2: IntoResponse,
{
    type Output = Result<R2>;

    async fn call(&self, req: Request) -> Self::Output {
        match self.inner.call(req).await {
            Ok(resp) => Ok((self.f)(resp).await),
            Err(err) => Err(err),
        }
    }
}
