use std::{error::Error as StdError, future::Future};

use bytes::Bytes;
use hyper::body::HttpBody;
use tower::{buffer::Buffer, make::Shared, MakeService, Service, ServiceExt};

use crate::{body::BodyStream, http::StatusCode, Endpoint, Error, Request, Response, Result};

/// Extension trait for tower compat.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub trait TowerCompatExt {
    /// Converts a tower service to a poem endpoint.
    fn compat<ResBody, Err, Fut>(self) -> TowerCompatEndpoint<Self, ResBody, Err, Fut>
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
            > + Send
            + Sized
            + 'static,
        Fut: Future<Output = Result<hyper::Response<ResBody>, Err>> + Send + 'static,
    {
        TowerCompatEndpoint(Shared::new(Buffer::new(self, 32)))
    }
}

impl<T> TowerCompatExt for T {}

/// A tower service adapter.
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
pub struct TowerCompatEndpoint<Svc, ResBody, Err, Fut>(
    Shared<Buffer<Svc, http::Request<hyper::Body>>>,
)
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
        > + Send
        + 'static,
    Fut: Future<Output = Result<hyper::Response<ResBody>, Err>> + Send + 'static;

#[async_trait::async_trait]
impl<Svc, ResBody, Err, Fut> Endpoint for TowerCompatEndpoint<Svc, ResBody, Err, Fut>
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
        > + Send
        + 'static,
    Fut: Future<Output = Result<hyper::Response<ResBody>, Err>> + Send + 'static,
{
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        let mut make = self.0.clone();
        let mut svc = MakeService::make_service(&mut make, ())
            .await
            .map_err(|err| Error::new(StatusCode::INTERNAL_SERVER_ERROR).with_reason_string(err))?;
        svc.ready()
            .await
            .map_err(|err| Error::new(StatusCode::INTERNAL_SERVER_ERROR).with_reason_string(err))?;

        let hyper_req: http::Request<hyper::Body> = req.into();
        let hyper_resp = svc
            .call(hyper_req.map(Into::into))
            .await
            .map_err(|err| Error::new(StatusCode::BAD_REQUEST).with_reason_string(err))?;

        Ok(hyper_resp
            .map(|body| hyper::Body::wrap_stream(BodyStream::new(body)))
            .into())
    }
}
