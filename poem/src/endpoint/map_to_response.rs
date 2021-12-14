use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`map_to_response`](super::EndpointExt::map_to_response)
/// method.
pub struct MapToResponse<E> {
    inner: E,
}

impl<E> MapToResponse<E> {
    #[inline]
    pub(crate) fn new(inner: E) -> MapToResponse<E> {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for MapToResponse<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.inner.call(req).await.map(IntoResponse::into_response)
    }
}
