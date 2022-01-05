use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`and_then`](super::EndpointExt::and_then) method.
pub struct AndThen<E, F> {
    inner: E,
    f: F,
}

impl<E, F> AndThen<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> AndThen<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R, R2> Endpoint for AndThen<E, F>
where
    E: Endpoint<Output = R>,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = Result<R2>> + Send,
    R: IntoResponse,
    R2: IntoResponse,
{
    type Output = R2;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let resp = self.inner.call(req).await?;
        (self.f)(resp).await
    }
}
