use crate::{Endpoint, Guard, Request, Response, Result};

/// Endpoint for the [`guard`](super::EndpointExt::guard) method.
pub struct GuardEndpoint<E, T> {
    inner: E,
    guard: T,
}

impl<E, T> GuardEndpoint<E, T> {
    pub(crate) fn new(inner: E, guard: T) -> Self {
        Self { guard, inner }
    }
}

#[async_trait::async_trait]
impl<E, T> Endpoint for GuardEndpoint<E, T>
where
    T: Guard,
    E: Endpoint,
{
    #[inline]
    fn check(&self, req: &Request) -> bool {
        if !self.guard.check(req) {
            return false;
        }
        self.inner.check(req)
    }

    async fn call(&self, req: Request) -> Result<Response> {
        self.inner.call(req).await
    }
}
