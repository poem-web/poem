use std::future::Future;

use crate::{Endpoint, IntoResponse, Request};

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
    F: Fn(E::Output) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    R: IntoResponse,
{
    type Output = R;

    async fn call(&self, req: Request) -> Self::Output {
        (self.f)(self.inner.call(req).await).await
    }
}
