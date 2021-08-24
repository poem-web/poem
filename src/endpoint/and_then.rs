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
    E: Endpoint<Output = Result<R>>,
    F: Fn(R) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R2>> + Send + 'static,
    R: IntoResponse,
    R2: IntoResponse,
{
    type Output = Result<R2>;

    async fn call(&self, req: Request) -> Self::Output {
        match self.inner.call(req).await {
            Ok(resp) => (self.f)(resp).await,
            Err(err) => Err(err),
        }
    }
}
