use std::future::Future;

use crate::{Endpoint, Request, Result};

/// Endpoint for the [`before`](super::EndpointExt::before) method.
pub struct Before<E, F> {
    inner: E,
    f: F,
}

impl<E, F> Before<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Before<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut> Endpoint for Before<E, F>
where
    E: Endpoint,
    F: Fn(Request) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Request>> + Send,
{
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.inner.call((self.f)(req).await?).await
    }
}
