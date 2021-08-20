//! Commonly used as the type of extractor or response.

mod cookie;
mod data;
mod form;
mod json;
#[cfg(feature = "multipart")]
mod multipart;
mod path;
mod query;
#[cfg(feature = "sse")]
#[cfg_attr(docsrs, doc(cfg(feature = "sse")))]
pub mod sse;
#[cfg(feature = "tempfile")]
mod tempfile;
#[cfg(feature = "typed-headers")]
mod typed_header;
#[cfg(feature = "websocket")]
#[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
pub mod websocket;

/// Commonly used typed headers.
#[cfg(feature = "typed-headers")]
#[cfg_attr(docsrs, doc(cfg(feature = "typed-headers")))]
pub mod type_headers {
    pub use typed_headers::{
        Accept, AcceptEncoding, Allow, AuthScheme, Authorization, ContentCoding, ContentEncoding,
        ContentLength, ContentType, Credentials, Host, HttpDate, ProxyAuthorization, Quality,
        QualityItem, RetryAfter, Token68,
    };
}

use std::convert::Infallible;

use bytes::Bytes;
pub use data::Data;
pub use form::Form;
pub use json::Json;
#[cfg(feature = "multipart")]
pub use multipart::{Field, Multipart};
pub use path::Path;
pub use query::Query;
#[cfg(feature = "typed-headers")]
#[cfg_attr(docsrs, doc(cfg(feature = "typed-headers")))]
pub use typed_header::TypedHeader;

pub use self::cookie::{Cookie, CookieJar};
#[cfg(feature = "tempfile")]
pub use self::tempfile::TempFile;
use crate::{
    body::Body,
    error::{Error, Result},
    http::{header::HeaderMap, Method, StatusCode, Uri, Version},
    request::Request,
    response::Response,
};

define_simple_errors! (
    /// This error occurs when the body has been taken.
    (ErrorBodyHasBeenTaken, INTERNAL_SERVER_ERROR, "the body has been taken");
);

/// The body parameter type of [`FromRequest::from_request`] method.
pub struct RequestBody(Option<Body>);

impl RequestBody {
    pub(crate) fn new(body: Option<Body>) -> Self {
        Self(body)
    }

    /// Take a body, if it has already been taken, an error with the status code
    /// [`INTERNAL_SERVER_ERROR`] is returned.
    pub fn take(&mut self) -> Result<Body> {
        self.0.take().ok_or_else(|| ErrorBodyHasBeenTaken.into())
    }
}

/// Types that can be created from requests.
#[async_trait::async_trait]
pub trait FromRequest<'a>: Sized {
    /// Perform the extraction.
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self>;
}

/// Trait for generating responses.
///
/// Types that implement [IntoResponse] can be returned from endpoints/handlers.
pub trait IntoResponse {
    /// Consume itself and return [`Response`].
    fn into_response(self) -> Result<Response>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(self))
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(self))
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(self))
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(self))
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(Body::empty()))
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().body(Body::empty()))
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder().status(self).finish())
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, T) {
    fn into_response(self) -> Result<Response> {
        let mut resp = self.1.into_response()?;
        resp.set_status(self.0);
        Ok(resp)
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, HeaderMap, T) {
    fn into_response(self) -> Result<Response> {
        let mut resp = self.2.into_response()?;
        resp.set_status(self.0);
        resp.headers_mut().extend(self.1.into_iter());
        Ok(resp)
    }
}

impl<T: IntoResponse, E: Into<Error>> IntoResponse for Result<T, E> {
    fn into_response(self) -> Result<Response> {
        self.map_err(Into::into)
            .and_then(IntoResponse::into_response)
    }
}

/// An HTML response.
pub struct Html<T>(pub T);

impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder()
            .content_type("text/html")
            .body(self.0.into()))
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a Request {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a Uri {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req.uri())
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Method {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req.method().clone())
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Version {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req.version())
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a HeaderMap {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req.headers())
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Body {
    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        body.take()
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for String {
    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        String::from_utf8(body.take()?.into_bytes().await?.to_vec()).map_err(Error::bad_request)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Bytes {
    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        Ok(body.take()?.into_bytes().await?)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Vec<u8> {
    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        Ok(body.take()?.into_bytes().await?.to_vec())
    }
}

#[async_trait::async_trait]
impl<'a, T: FromRequest<'a>> FromRequest<'a> for Option<T> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        Ok(T::from_request(req, body).await.ok())
    }
}
