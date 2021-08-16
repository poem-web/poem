use crate::{Endpoint, Error, Request, Response, Result};

/// Endpoint for the [`map_err`](super::EndpointExt::map_err) method.
pub struct MapErr<E, F> {
    inner: E,
    f: F,
}

impl<E, F> MapErr<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> MapErr<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F> Endpoint for MapErr<E, F>
where
    E: Endpoint,
    F: Fn(Error) -> Error + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self.inner.call(req).await.map_err(&self.f)
    }
}
