use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Response, Result};

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
impl<E, F, Fut, R> Endpoint for AndThen<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R>> + Send + 'static,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => (self.f)(resp)
                .await
                .map_err(Into::into)
                .and_then(IntoResponse::into_response),
            Err(err) => Err(err),
        }
    }
}
