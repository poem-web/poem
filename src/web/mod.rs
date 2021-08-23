//! Commonly used as the type of extractor or response.

#[cfg(feature = "compression")]
mod compress;
mod cookie;
mod data;
mod form;
mod json;
#[cfg(feature = "multipart")]
mod multipart;
mod path;
mod query;
mod redirect;
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

use std::convert::{Infallible, TryInto};

use bytes::Bytes;
#[cfg(feature = "compression")]
pub use compress::{Compress, CompressionAlgo};
pub use data::Data;
pub use form::Form;
pub use json::Json;
#[cfg(feature = "multipart")]
pub use multipart::{Field, Multipart};
pub use path::Path;
pub use query::Query;
pub use redirect::Redirect;
#[cfg(feature = "typed-headers")]
#[cfg_attr(docsrs, doc(cfg(feature = "typed-headers")))]
pub use typed_header::TypedHeader;

pub use self::cookie::{Cookie, CookieJar};
#[cfg(feature = "tempfile")]
pub use self::tempfile::TempFile;
use crate::{
    body::Body,
    error::{Error, Result},
    http::{
        header::{HeaderMap, HeaderName},
        HeaderValue, Method, StatusCode, Uri, Version,
    },
    request::Request,
    response::Response,
};

/// The body parameter type of [`FromRequest::from_request`] method.
pub struct RequestBody(Option<Body>);

impl RequestBody {
    pub(crate) fn new(body: Option<Body>) -> Self {
        Self(body)
    }

    /// Take a body, if it has already been taken, an error with the status code
    /// [`StatusCode::INTERNAL_SERVER_ERROR`] is returned.
    pub fn take(&mut self) -> Result<Body> {
        self.0
            .take()
            .ok_or_else(|| Error::internal_server_error("the body has been taken"))
    }
}

/// Represents an type that can be extract from requests.
///
/// # Provided Implementations
///
/// - **Option&lt;T>**
///
///    Extracts `T` from the incoming request, returns [`None`] if it
/// fails.
///
/// - **&Request**
///
///    Extracts the [`Request`] from the incoming request.
///
/// - **Method**
///
///    Extracts the [`Method`] from the incoming request.
///
/// - **Version**
///
///    Extracts the [`Version`] from the incoming request.
///
/// - **&Uri**
///
///    Extracts the [`Uri`] from the incoming request.
///
/// - **&HeaderMap**
///
///    Extracts the [`HeaderMap`] from the incoming request.
///
/// - **Data&lt;&T>**
///
///    Extracts the [`Data`] from the incoming request.
///
/// - **TypedHeader&lt;T>**
///
///    Extracts the [`TypedHeader`] from the incoming request.
///
/// - **Path&lt;T>**
///
///    Extracts the [`Path`] from the incoming request.
///
/// - **Query&lt;T>**
///
///    Extracts the [`Query`] from the incoming request.
///
/// - **Form&lt;T>**
///
///    Extracts the [`Form`] from the incoming request.
///
/// - **Json&lt;T>**
///
///    Extracts the [`Json`] from the incoming request.
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **TempFile**
///
///    Extracts the [`TempFile`] from the incoming request.
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **Multipart**
///
///    Extracts the [`Multipart`] from the incoming request.
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **Body**
///
///    Extracts the [`Body`] from the incoming request.
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **String**
///
///    Extracts the body from the incoming request and parse it into utf8
/// [`String].
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **Vec&lt;u8>**
///
///    Extracts the body from the incoming request and collect it into
/// [`Vec<u8>`].
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **Bytes**
///
///    Extracts the body from the incoming request and collect it into
/// [`Bytes`].
///
///    _This extractor will take over the requested body, so you should avoid
/// using multiple extractors of this type in one handler._
///
/// - **WebSocket**
///
///    Ready to accept a websocket [`WebSocket`](websocket::WebSocket)
/// connection.
///
/// # Custom extractor
///
/// The following is an example of a custom token extractor, which extracts the
/// token from the `MyToken` header.
///
/// ```
/// use poem::{handler, route, Error, FromRequest, Request, RequestBody};
///
/// struct Token(String);
///
/// #[poem::async_trait]
/// impl<'a> FromRequest<'a> for Token {
///     async fn from_request(req: &'a Request, body: &mut RequestBody) -> poem::Result<Self> {
///         let token = req
///             .headers()
///             .get("MyToken")
///             .and_then(|value| value.to_str().ok())
///             .ok_or_else(|| Error::bad_request("missing token"))?;
///         Ok(Token(token.to_string()))
///     }
/// }
///
/// #[handler]
/// async fn index(token: Token) {
///     println!("Token: {}", token.0);
/// }
///
/// let app = route().at("/", index);
/// ```

#[async_trait::async_trait]
pub trait FromRequest<'a>: Sized {
    /// Perform the extraction.
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self>;
}

/// Represents a type that can convert into response.
///
/// # Provided Implementations
///
/// - **()**
///
///    Sets the status to `OK` with an empty body.
///
/// - **&'static str**
///
///    Sets the status to `OK` and the `Content-Type` to `text/plain`. The
/// string is used as the body of the response.
///
/// - **String**
///
///    Sets the status to `OK` and the `Content-Type` to `text/plain`. The
/// string is used as the body of the response.
///
/// - **&'static [u8]**
///
///    Sets the status to `OK` and the `Content-Type` to
/// `application/octet-stream`. The slice is used as the body of the response.
///
/// - **Html&lt;T>**
///
///    Sets the status to `OK` and the `Content-Type` to `text/html`. `T` is
/// used as the body of the response.
///
/// - **Json&lt;T>**
///
///    Sets the status to `OK` and the `Content-Type` to `application/json`. Use
/// [`serde_json`](https://crates.io/crates/serde_json) to serialize `T` into a json string.
///
/// - **Bytes**
///
///    Sets the status to `OK` and the `Content-Type` to
/// `application/octet-stream`. The bytes is used as the body of the response.
///
/// - **Vec&lt;u8>**
///
///    Sets the status to `OK` and the `Content-Type` to
/// `application/octet-stream`. The vector’s data is used as the body of the
/// response.
///
/// - **StatusCode**
///
///    Sets the status to the specified status code [`StatusCode`] with an empty
/// body.
///
/// - **(StatusCode, T)**
///
///    Convert `T` to response and set the specified status code [`StatusCode`].
///
/// - **(StatusCode, HeaderMap, T)**
///
///    Convert `T` to response and set the specified status code [`StatusCode`],
/// and then merge the specified [`HeaderMap`].
///
/// - **Response**
///
///    The implementation for [`Response`] always returns itself.
///
/// - **Compress&lt;T>**
///
///    Call `T::into_response` to get the response, then compress the response
/// body with the specified algorithm, and set the correct `Content-Encoding`
/// header.
///
/// - **SSE**
///
///     Sets the status to `OK` and the `Content-Type` to `text/event-stream`
/// with an event stream body. Use the [`SSE::new`](sse::SSE::new) function to
/// create it.

pub trait IntoResponse {
    /// Consume itself and return [`Response`].
    fn into_response(self) -> Response;

    /// Wrap an `impl IntoResponse` to add a header.
    fn with_header<K, V>(self, key: K, value: V) -> WithHeader<Self>
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
        Self: Sized,
    {
        let key = key.try_into().ok();
        let value = value.try_into().ok();

        WithHeader {
            inner: self,
            header: key.zip(value),
        }
    }

    /// Wrap an `impl IntoResponse` to set a status code.
    fn with_status(self, status: StatusCode) -> WithStatus<Self>
    where
        Self: Sized,
    {
        WithStatus {
            inner: self,
            status,
        }
    }

    /// Wrap an `impl IntoResponse` to set a body.
    fn with_body(self, body: impl Into<Body>) -> WithBody<Self>
    where
        Self: Sized,
    {
        WithBody {
            inner: self,
            body: body.into(),
        }
    }
}

/// Returned by [`with_header`](IntoResponse::with_header) method.
pub struct WithHeader<T> {
    inner: T,
    header: Option<(HeaderName, HeaderValue)>,
}

impl<T: IntoResponse> IntoResponse for WithHeader<T> {
    fn into_response(self) -> Response {
        let mut resp = self.inner.into_response();
        if let Some((key, value)) = &self.header {
            resp.headers_mut().append(key.clone(), value.clone());
        }
        resp
    }
}

/// Returned by [`with_header`](IntoResponse::with_status) method.
pub struct WithStatus<T> {
    inner: T,
    status: StatusCode,
}

impl<T: IntoResponse> IntoResponse for WithStatus<T> {
    fn into_response(self) -> Response {
        let mut resp = self.inner.into_response();
        resp.set_status(self.status);
        resp
    }
}

/// Returned by [`with_body`](IntoResponse::with_body) method.
pub struct WithBody<T> {
    inner: T,
    body: Body,
}

impl<T: IntoResponse> IntoResponse for WithBody<T> {
    fn into_response(self) -> Response {
        let mut resp = self.inner.into_response();
        resp.set_body(self.body);
        resp
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::builder().content_type("text/plain").body(self)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Response::builder().content_type("text/plain").body(self)
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type("application/octet-stream")
            .body(self)
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type("application/octet-stream")
            .body(self)
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type("application/octet-stream")
            .body(self)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::builder().body(Body::empty())
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response {
        Response::builder().body(Body::empty())
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        Response::builder().status(self).finish()
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, T) {
    fn into_response(self) -> Response {
        let mut resp = self.1.into_response();
        resp.set_status(self.0);
        resp
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, HeaderMap, T) {
    fn into_response(self) -> Response {
        let mut resp = self.2.into_response();
        resp.set_status(self.0);
        resp.headers_mut().extend(self.1.into_iter());
        resp
    }
}

impl<T: IntoResponse> IntoResponse for Result<T> {
    fn into_response(self) -> Response {
        match self {
            Ok(resp) => resp.into_response(),
            Err(err) => err.as_response(),
        }
    }
}

/// An HTML response.
pub struct Html<T>(pub T);

impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type("text/html")
            .body(self.0.into())
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
        let data = body.take()?.into_bytes().await?;
        String::from_utf8(data.to_vec()).map_err(Error::bad_request)
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
