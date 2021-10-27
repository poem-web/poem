use std::{error::Error as StdError, future::Future};

use bytes::Bytes;
use hyper::body::HttpBody;
use tower::{Service, ServiceExt};

use crate::{body::BodyStream, Endpoint, Request, Response, Result};

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
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        let mut svc = self.0.clone();

        svc.ready().await?;

        let hyper_req: http::Request<hyper::Body> = req.into();
        let hyper_resp = svc.call(hyper_req.map(Into::into)).await?;

        Ok(hyper_resp
            .map(|body| hyper::Body::wrap_stream(BodyStream::new(body)))
            .into())
    }
}
