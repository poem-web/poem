use crate::prelude::*;

/// Middleware for add any data to request.
pub struct AddData<T> {
    value: T,
}

impl<T: Clone + Send + Sync + 'static> AddData<T> {
    /// Create new [AddData] middleware with any value.
    pub fn new(value: T) -> Self {
        AddData { value }
    }
}

impl<E, T> Middleware<E> for AddData<T>
where
    E: Endpoint,
    T: Clone + Send + Sync + 'static,
{
    type Output = AddDataImpl<E, T>;

    fn transform(&self, ep: E) -> Self::Output {
        AddDataImpl {
            inner: ep,
            value: self.value.clone(),
        }
    }
}

#[doc(hidden)]
pub struct AddDataImpl<E, T> {
    inner: E,
    value: T,
}

#[async_trait::async_trait]
impl<E, T> Endpoint for AddDataImpl<E, T>
where
    E: Endpoint,
    T: Clone + Send + Sync + 'static,
{
    async fn call(&self, mut req: Request) -> Result<Response> {
        req.extensions_mut().insert(self.value.clone());
        self.inner.call(req).await
    }
}
