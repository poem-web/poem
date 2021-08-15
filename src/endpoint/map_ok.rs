use super::Endpoint;
use crate::{error::Result, request::Request, response::Response};

/// Endpoint for the [`map_ok`](super::EndpointExt::map_ok) method.
pub struct MapOk<E, F> {
    inner: E,
    f: F,
}

impl<E, F> MapOk<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> MapOk<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F> Endpoint for MapOk<E, F>
where
    E: Endpoint,
    F: Fn(Response) -> Response + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        Ok((self.f)(self.inner.call(req).await?))
    }
}
