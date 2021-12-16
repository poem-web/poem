use std::marker::PhantomData;

use crate::{Endpoint, Request, Result};

/// Endpoint for the
/// [`inspect_typed_err`](super::EndpointExt::inspect_typed_err) method.
pub struct InspectTypedError<E, F, ErrType> {
    inner: E,
    f: F,
    _mark: PhantomData<ErrType>,
}

impl<E, F, ErrType> InspectTypedError<E, F, ErrType> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> InspectTypedError<E, F, ErrType> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<E, F, ErrType> Endpoint for InspectTypedError<E, F, ErrType>
where
    E: Endpoint,
    F: Fn(&ErrType) + Send + Sync,
    ErrType: std::error::Error + Send + Sync + 'static,
{
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp),
            Err(err) if err.is::<ErrType>() => {
                (self.f)(err.downcast_ref::<ErrType>().unwrap());
                Err(err)
            }
            Err(err) => Err(err),
        }
    }
}
