use std::{error::Error as StdError, future::Future};

use bytes::Bytes;
use http_body_util::BodyExt;
use tower::{Service, ServiceExt};

use crate::{body::BoxBody, Endpoint, Error, Request, Response, Result};

/// Extension trait for tower service compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerCompatExt {
    /// Converts a tower service to a poem endpoint.
    fn compat<ResBody, Err, Fut>(self) -> TowerCompatEndpoint<Self>
    where
        ResBody: hyper::body::Body + Send + Sync + 'static,
        ResBody::Data: Into<Bytes> + Send + 'static,
        ResBody::Error: StdError + Send + Sync + 'static,
        Err: Into<Error>,
        Self: Service<
                http::Request<BoxBody>,
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

impl<Svc, ResBody, Err, Fut> Endpoint for TowerCompatEndpoint<Svc>
where
    ResBody: hyper::body::Body + Send + Sync + 'static,
    ResBody::Data: Into<Bytes> + Send + 'static,
    ResBody::Error: StdError + Send + Sync + 'static,
    Err: Into<Error>,
    Svc: Service<
            http::Request<BoxBody>,
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

        svc.ready().await.map_err(Into::into)?;
        svc.call(req.into()).await.map_err(Into::into).map(|resp| {
            let (parts, body) = resp.into_parts();
            let body = body
                .map_frame(|frame| frame.map_data(Into::into))
                .map_err(std::io::Error::other)
                .boxed();
            hyper::Response::from_parts(parts, body).into()
        })
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
    use crate::test::TestClient;

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
        let resp = TestClient::new(ep).get("/").body("abc").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("abc").await;
    }
}
