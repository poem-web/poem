use std::{error::Error as StdError, future::Future};

use bytes::Bytes;
use hyper::body::{HttpBody, Sender};
use tower::{Service, ServiceExt};

use crate::{Endpoint, Error, Request, Response, Result};

/// Extension trait for tower service compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerCompatExt {
    /// Converts a tower service to a poem endpoint.
    fn compat<ResBody, Err, Fut>(self) -> TowerCompatEndpoint<Self>
    where
        ResBody: HttpBody + Send + 'static,
        ResBody::Data: Into<Bytes> + Send + 'static,
        ResBody::Error: StdError + Send + Sync + 'static,
        Err: Into<Error>,
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
    Err: Into<Error>,
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

        svc.ready().await.map_err(Into::into)?;

        let hyper_req: http::Request<hyper::Body> = req.into();
        let hyper_resp = svc
            .call(hyper_req.map(Into::into))
            .await
            .map_err(Into::into)?;

        if !hyper_resp.body().is_end_stream() {
            Ok(hyper_resp
                .map(|body| {
                    let (sender, new_body) = hyper::Body::channel();
                    tokio::spawn(copy_body(body, sender));
                    new_body
                })
                .into())
        } else {
            Ok(hyper_resp.map(|_| hyper::Body::empty()).into())
        }
    }
}

async fn copy_body<T>(body: T, mut sender: Sender)
where
    T: HttpBody + Send + 'static,
    T::Data: Into<Bytes> + Send + 'static,
    T::Error: StdError + Send + Sync + 'static,
{
    tokio::pin!(body);

    loop {
        match body.data().await {
            Some(Ok(data)) => {
                if sender.send_data(data.into()).await.is_err() {
                    break;
                }
            }
            Some(Err(_)) => break,
            None => {}
        }

        match body.trailers().await {
            Ok(Some(trailers)) => {
                if sender.send_trailers(trailers).await.is_err() {
                    break;
                }
            }
            Ok(None) => {}
            Err(_) => break,
        }

        if body.is_end_stream() {
            break;
        }
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
