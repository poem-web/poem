use crate::{Endpoint, Request, Response, Result};

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
impl<E> Endpoint for MapToResponse<E>
where
    E: Endpoint,
{
    async fn call(&self, req: Request) -> Result<Response> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp),
            Err(err) => Ok(err.as_response()),
        }
    }
}
