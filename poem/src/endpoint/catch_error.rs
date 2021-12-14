use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`catch_error`](super::EndpointExt::catch_error) method.
pub struct CatchError<E, F, ErrType> {
    inner: E,
    f: F,
    _mark: PhantomData<ErrType>,
}

impl<E, F, ErrType> CatchError<E, F, ErrType> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> CatchError<E, F, ErrType> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, ErrType> Endpoint for CatchError<E, F, ErrType>
where
    E: Endpoint,
    F: Fn(ErrType) -> Fut + Send + Sync,
    Fut: Future<Output = Response> + Send,
    ErrType: std::error::Error + Send + Sync + 'static,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp.into_response()),
            Err(err) if err.is::<ErrType>() => {
                Ok((self.f)(err.downcast::<ErrType>().unwrap()).await)
            }
            Err(err) => Err(err),
        }
    }
}
