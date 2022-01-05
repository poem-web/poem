use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`map_ok`](super::EndpointExt::map) method.
pub struct Map<E, F> {
    inner: E,
    f: F,
}

impl<E, F> Map<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Map<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R, R2> Endpoint for Map<E, F>
where
    E: Endpoint<Output = R>,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = R2> + Send,
    R: IntoResponse,
    R2: IntoResponse,
{
    type Output = R2;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let resp = self.inner.call(req).await?;
        Ok((self.f)(resp).await)
    }
}
