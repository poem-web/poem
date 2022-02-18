use crate::{Endpoint, Request, Response, Result};

/// Endpoint for the [`to_response`](super::EndpointExt::to_response)
/// method.
pub struct ToResponse<E> {
    inner: E,
}

impl<E> ToResponse<E> {
    #[inline]
    pub(crate) fn new(inner: E) -> ToResponse<E> {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for ToResponse<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        Ok(self.inner.get_response(req).await)
    }
}
