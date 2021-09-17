use crate::{Endpoint, Error, IntoResponse, Request, Response, Result};

/// Endpoint for the [`map_to_result`](super::EndpointExt::map_to_result)
/// method.
pub struct MapToResult<E> {
    inner: E,
}

impl<E> MapToResult<E> {
    #[inline]
    pub(crate) fn new(inner: E) -> MapToResult<E> {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for MapToResult<E> {
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        let resp = self.inner.call(req).await.into_response();
        if !resp.status().is_server_error() && !resp.status().is_client_error() {
            Ok(resp)
        } else {
            Err(Error::new(resp.status()))
        }
    }
}
