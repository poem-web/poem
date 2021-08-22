use std::future::Future;

use crate::{Endpoint, IntoResponse, Request, Response};

/// Endpoint for the [`after`](super::EndpointExt::after) method.
pub struct After<E, F> {
    inner: E,
    f: F,
}

impl<E, F> After<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> After<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R> Endpoint for After<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Response {
        (self.f)(self.inner.call(req).await).await.into_response()
    }
}
