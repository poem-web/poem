use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::{future::BoxFuture, FutureExt};
use tower::{buffer::Buffer, BoxError, Layer, Service, ServiceExt};

use crate::{Endpoint, Error, IntoResponse, Middleware, Request, Result};

#[doc(hidden)]
#[derive(Debug, thiserror::Error)]
#[error("wrapper error")]
pub struct WrapperError(Error);

fn boxed_err_to_poem_err(err: BoxError) -> Error {
    match err.downcast::<WrapperError>() {
        Ok(err) => (*err).0,
        Err(err) => Error::new_with_string(err.to_string()),
    }
}

/// Extension trait for tower layer compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerLayerCompatExt {
    /// Converts a tower layer to a poem middleware.
    fn compat(self) -> TowerCompatMiddleware<Self>
    where
        Self: Sized,
    {
        TowerCompatMiddleware(self)
    }
}

impl<L> TowerLayerCompatExt for L {}

/// A tower layer adapter.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub struct TowerCompatMiddleware<L>(L);

impl<E, L> Middleware<E> for TowerCompatMiddleware<L>
where
    E: Endpoint,
    L: Layer<EndpointToTowerService<E>>,
    L::Service: Service<Request> + Send + 'static,
    <L::Service as Service<Request>>::Future: Send,
    <L::Service as Service<Request>>::Response: IntoResponse + Send + 'static,
    <L::Service as Service<Request>>::Error: Into<BoxError> + Send + Sync,
{
    type Output = TowerServiceToEndpoint<L::Service>;

    fn transform(&self, ep: E) -> Self::Output {
        let new_svc = self.0.layer(EndpointToTowerService(Arc::new(ep)));
        let buffer = Buffer::new(new_svc, 32);
        TowerServiceToEndpoint(buffer)
    }
}

/// An endpoint to tower service adapter.
pub struct EndpointToTowerService<E>(Arc<E>);

impl<E> Service<Request> for EndpointToTowerService<E>
where
    E: Endpoint + 'static,
{
    type Response = E::Output;
    type Error = WrapperError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let ep = self.0.clone();
        async move { ep.call(req).await.map_err(WrapperError) }.boxed()
    }
}

/// An tower service to endpoint adapter.
pub struct TowerServiceToEndpoint<Svc: Service<Request>>(Buffer<Svc, Request>);

#[async_trait::async_trait]
impl<Svc> Endpoint for TowerServiceToEndpoint<Svc>
where
    Svc: Service<Request> + Send + 'static,
    Svc::Future: Send,
    Svc::Response: IntoResponse + 'static,
    Svc::Error: Into<BoxError> + Send + Sync,
{
    type Output = Svc::Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut svc = self.0.clone();
        svc.ready().await.map_err(boxed_err_to_poem_err)?;
        let res = svc.call(req).await.map_err(boxed_err_to_poem_err)?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{endpoint::make_sync, EndpointExt};

    #[tokio::test]
    async fn test_tower_layer() {
        struct TestService<S> {
            inner: S,
        }

        impl<S, Req> Service<Req> for TestService<S>
        where
            S: Service<Req>,
        {
            type Response = S::Response;
            type Error = S::Error;
            type Future = S::Future;

            fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                self.inner.poll_ready(cx)
            }

            fn call(&mut self, req: Req) -> Self::Future {
                self.inner.call(req)
            }
        }

        struct MyServiceLayer;

        impl<S> Layer<S> for MyServiceLayer {
            type Service = TestService<S>;

            fn layer(&self, inner: S) -> Self::Service {
                TestService { inner }
            }
        }

        let ep = make_sync(|_| ()).with(MyServiceLayer.compat());
        let resp = ep.call(Request::default()).await.unwrap().into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
