use crate::{Endpoint, Error, Request, Result};

/// Endpoint for the [`inspect_all_err`](super::EndpointExt::inspect_all_err)
/// method.
pub struct InspectAllError<E, F> {
    inner: E,
    f: F,
}

impl<E, F> InspectAllError<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> InspectAllError<E, F> {
        Self { inner, f }
    }
}

#[async_trait::async_trait]
impl<E, F> Endpoint for InspectAllError<E, F>
where
    E: Endpoint,
    F: Fn(&Error) + Send + Sync,
{
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.inner.call(req).await {
            Ok(resp) => Ok(resp),
            Err(err) => {
                (self.f)(&err);
                Err(err)
            }
        }
    }
}
