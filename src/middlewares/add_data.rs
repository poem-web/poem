use crate::{Endpoint, Middleware, Request, Response, Result};

pub struct AddData<D> {
    value: D,
}

impl<D: Clone + Send + Sync + 'static> AddData<D> {
    pub fn new(value: D) -> Self {
        AddData { value }
    }
}

impl<D: Clone + Send + Sync + 'static> Middleware for AddData<D> {
    fn transform<T: Endpoint>(&self, ep: T) -> Box<dyn Endpoint> {
        Box::new(AddDataImpl {
            inner: ep,
            value: self.value.clone(),
        })
    }
}

struct AddDataImpl<E, T> {
    inner: E,
    value: T,
}

#[async_trait::async_trait]
impl<E: Endpoint, T: Clone + Send + Sync + 'static> Endpoint for AddDataImpl<E, T> {
    async fn call(&self, mut req: Request) -> Result<Response> {
        req.extensions_mut().insert(self.value.clone());
        self.inner.call(req).await
    }
}
