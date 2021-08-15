use super::Endpoint;
use crate::{error::Result, request::Request, response::Response};

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
impl<E, F> Endpoint for Before<E, F>
where
    E: Endpoint,
    F: Fn(Request) -> Result<Request> + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self.inner.call((self.f)(req)?).await
    }
}
