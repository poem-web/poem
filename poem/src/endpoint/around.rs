use std::{future::Future, sync::Arc};

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`around`](super::EndpointExt::around) method.
pub struct Around<E, F> {
    inner: Arc<E>,
    f: F,
}

impl<E, F> Around<E, F> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Around<E, F> {
        Self {
            inner: Arc::new(inner),
            f,
        }
    }
}

#[async_trait::async_trait]
impl<E, F, Fut, T> Endpoint for Around<E, F>
where
    E: Endpoint,
    F: Fn(Arc<E>, Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T>> + Send,
    T: IntoResponse,
{
    type Output = T;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        (self.f)(self.inner.clone(), req).await
    }
}
