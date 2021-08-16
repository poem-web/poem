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
impl<E, F> Endpoint for AndThen<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Result<Response> + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self.inner.call(req).await.and_then(|resp| (self.f)(resp))
    }
}
