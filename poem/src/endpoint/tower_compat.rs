use std::{error::Error as StdError, future::Future, marker::PhantomData};

use bytes::Bytes;
use hyper::body::HttpBody;
use tower::{Service, ServiceExt};

use crate::{body::BodyStream, Endpoint, Error, Request, Response, Result};

/// Extension trait for tower service compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerCompatExt {
    /// Converts a tower service to a poem endpoint.
    fn compat<Req, Resp, Err, Fut>(self) -> TowerCompatEndpoint<Req, Self>
    where
        Req: From<Request> + Send + Sync,
        Resp: Into<Response>,
        Err: Into<Error>,
        Fut: Future<Output = Result<Resp, Err>> + Send + 'static,
        Self: Service<Req, Response = Resp, Error = Err, Future = Fut>
            + Clone
            + Send
            + Sync
            + Sized
            + 'static,
    {
        TowerCompatEndpoint {
            marker: PhantomData,
            svc: self,
        }
    }
}

impl<T> TowerCompatExt for T {}

/// A tower service adapter.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub struct TowerCompatEndpoint<Req, Svc> {
    marker: PhantomData<Req>,
    svc: Svc,
}

#[async_trait::async_trait]
impl<Svc, Req, Resp, Err, Fut> Endpoint for TowerCompatEndpoint<Req, Svc>
where
    Req: From<Request> + Send + Sync,
    Resp: Into<Response>,
    Err: Into<Error>,
    Fut: Future<Output = Result<Resp, Err>> + Send + 'static,
    Svc: Service<Req, Response = Resp, Error = Err, Future = Fut> + Clone + Send + Sync + 'static,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut svc = self.svc.clone();

        svc.ready().await.map_err(Into::into)?;

        let req: Req = req.into();
        svc.call(req).await.map(Into::into).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        num::ParseIntError,
        task::{Context, Poll},
    };

    use futures_util::future::Ready;

    use super::*;

    #[tokio::test]
    async fn test_tower_compat() {
        #[derive(Clone)]
        struct MyTowerService;

        impl<B> Service<http::Request<B>> for MyTowerService {
            type Response = http::Response<B>;
            type Error = Infallible;
            type Future = Ready<Result<Self::Response, Self::Error>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, req: http::Request<B>) -> Self::Future {
                futures_util::future::ready(Ok(http::Response::new(req.into_body())))
            }
        }

        let ep = MyTowerService.compat();
        let resp = ep.call(Request::builder().body("abc")).await.unwrap();
        assert_eq!(resp.into_body().into_string().await.unwrap(), "abc");
    }

    #[tokio::test]
    async fn test_map() {
        #[derive(Clone)]
        struct MyTowerService;

        impl Service<&str> for MyTowerService {
            type Response = i32;
            type Error = ParseIntError;
            type Future = Ready<Result<Self::Response, Self::Error>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, req: &str) -> Self::Future {
                futures_util::future::ready(req.parse())
            }
        }

        let ep =
            ServiceExt::map_request(MyTowerService, |req| Request::builder().body(req)).compat();
    }
}
