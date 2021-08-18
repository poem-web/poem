use std::future::Future;

use crate::{Endpoint, Request, Response, Result};

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
impl<E, F, Fut> Endpoint for AndThen<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => (self.f)(resp).await,
            Err(err) => Err(err),
        }
    }
}
