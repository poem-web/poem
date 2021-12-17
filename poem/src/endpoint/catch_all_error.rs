use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, Error, IntoResponse, Request, Response, Result};

/// Endpoint for the [`catch_all_error`](super::EndpointExt::catch_all_error)
/// method.
pub struct CatchAllError<E, F, R> {
    inner: E,
    f: F,
    _mark: PhantomData<R>,
}

impl<E, F, R> CatchAllError<E, F, R> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> CatchAllError<E, F, R> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R> Endpoint for CatchAllError<E, F, R>
where
    E: Endpoint,
    F: Fn(Error) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    R: IntoResponse + Send + Sync,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp.into_response()),
            Err(err) => Ok((self.f)(err).await.into_response()),
        }
    }
}
