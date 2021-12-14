use std::{error::Error as StdError, future::Future};

use bytes::Bytes;
use hyper::body::HttpBody;
use tower::{Service, ServiceExt};

use crate::{body::BodyStream, error::InternalServerError, Endpoint, Request, Response, Result};

/// Extension trait for tower service compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerCompatExt {
    /// Converts a tower service to a poem endpoint.
    fn compat<ResBody, Err, Fut>(self) -> TowerCompatEndpoint<Self>
    where
        ResBody: HttpBody + Send + 'static,
        ResBody::Data: Into<Bytes> + Send + 'static,
        ResBody::Error: StdError + Send + Sync + 'static,
        Err: StdError + Send + Sync + 'static,
        Self: Service<
                http::Request<hyper::Body>,
                Response = hyper::Response<ResBody>,
                Error = Err,
                Future = Fut,
            > + Clone
            + Send
            + Sync
            + Sized
            + 'static,
        Fut: Future<Output = Result<hyper::Response<ResBody>, Err>> + Send + 'static,
    {
        TowerCompatEndpoint(self)
    }
}

impl<T> TowerCompatExt for T {}

/// A tower service adapter.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub struct TowerCompatEndpoint<Svc>(Svc);

#[async_trait::async_trait]
impl<Svc, ResBody, Err, Fut> Endpoint for TowerCompatEndpoint<Svc>
where
    ResBody: HttpBody + Send + 'static,
    ResBody::Data: Into<Bytes> + Send + 'static,
    ResBody::Error: StdError + Send + Sync + 'static,
    Err: StdError + Send + Sync + 'static,
    Svc: Service<
            http::Request<hyper::Body>,
            Response = hyper::Response<ResBody>,
            Error = Err,
            Future = Fut,
        > + Clone
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Result<hyper::Response<ResBody>, Err>> + Send + 'static,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut svc = self.0.clone();

        svc.ready().await.map_err(InternalServerError)?;

        let hyper_req: http::Request<hyper::Body> = req.into();
        let hyper_resp = svc
            .call(hyper_req.map(Into::into))
            .await
            .map_err(InternalServerError)?;

        Ok(hyper_resp
            .map(|body| hyper::Body::wrap_stream(BodyStream::new(body)))
            .into())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
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
}
