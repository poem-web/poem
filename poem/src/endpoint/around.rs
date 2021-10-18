use futures_util::future::BoxFuture;

use crate::{Endpoint, IntoResponse, Request};

/// Endpoint for the [`around`](super::EndpointExt::around) method.
pub struct Around<E, F> {
    inner: E,
    f: F,
}

impl<E, F> Around<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Around<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F, R> Endpoint for Around<E, F>
where
    E: Endpoint,
    F: for<'a> Fn(&'a E, Request) -> BoxFuture<'a, R> + Send + Sync,
    R: IntoResponse,
{
    type Output = R;

    async fn call(&self, req: Request) -> Self::Output {
        (self.f)(&self.inner, req).await
    }
}
