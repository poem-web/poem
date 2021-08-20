use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`map`](super::EndpointExt::map) method.
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
impl<E, F, Fut, R> Endpoint for Map<E, F>
where
    E: Endpoint,
    F: Fn(Result<Response>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R>> + Send + 'static,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response> {
        (self.f)(self.inner.call(req).await)
            .await
            .and_then(IntoResponse::into_response)
    }
}
