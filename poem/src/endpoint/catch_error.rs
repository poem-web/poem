use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`catch_error`](super::EndpointExt::catch_error) method.
pub struct CatchError<E, F, R, ErrType> {
    inner: E,
    f: F,
    _mark1: PhantomData<R>,
    _mark2: PhantomData<ErrType>,
}

impl<E, F, R, ErrType> CatchError<E, F, R, ErrType> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> CatchError<E, F, R, ErrType> {
        Self {
            inner,
            f,
            _mark1: PhantomData,
            _mark2: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, R, ErrType> Endpoint for CatchError<E, F, R, ErrType>
where
    E: Endpoint,
    F: Fn(ErrType) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    R: IntoResponse + Send + Sync,
    ErrType: std::error::Error + Send + Sync + 'static,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp.into_response()),
            Err(err) if err.is::<ErrType>() => Ok((self.f)(err.downcast::<ErrType>().unwrap())
                .await
                .into_response()),
            Err(err) => Err(err),
        }
    }
}
