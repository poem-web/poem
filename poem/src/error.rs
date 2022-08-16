//! Some common error types.

use std::{
    convert::Infallible,
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    string::FromUtf8Error,
};

use headers::{ContentRange, HeaderMapExt};
use http::{Extensions, Method};

use crate::{http::StatusCode, IntoResponse, Response};

macro_rules! define_http_error {
    ($($(#[$docs:meta])* ($name:ident, $status:ident);)*) => {
        $(
        $(#[$docs])*
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name(err: impl StdError + Send + Sync + 'static) -> Error {
            Error::new(err, StatusCode::$status)
        }
        )*
    };
}

/// Represents a type that can be converted to [`Error`].
pub trait ResponseError {
    /// The status code of this error.
    fn status(&self) -> StatusCode;

    /// Convert this error to a HTTP response.
    fn as_response(&self) -> Response
    where
        Self: StdError + Send + Sync + 'static,
    {
        Response::builder()
            .status(self.status())
            .body(self.to_string())
    }
}

enum ErrorSource {
    BoxedError(Box<dyn StdError + Send + Sync>),
    #[cfg(feature = "anyhow")]
    Anyhow(anyhow::Error),
    #[cfg(feature = "eyre06")]
    Eyre06(eyre06::Report),
}

impl Debug for ErrorSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSource::BoxedError(err) => Debug::fmt(err, f),
            #[cfg(feature = "anyhow")]
            ErrorSource::Anyhow(err) => Debug::fmt(err, f),
            #[cfg(feature = "eyre06")]
            ErrorSource::Eyre06(err) => Debug::fmt(err, f),
        }
    }
}

type BoxAsResponseFn = Box<dyn Fn(&Error) -> Response + Send + Sync + 'static>;

enum AsResponse {
    Status(StatusCode),
    Fn(BoxAsResponseFn),
    Response(Response),
}

impl AsResponse {
    #[inline]
    fn from_status(status: StatusCode) -> Self {
        AsResponse::Status(status)
    }

    fn from_type<T: ResponseError + StdError + Send + Sync + 'static>() -> Self {
        AsResponse::Fn(Box::new(|err| {
            let err = err.downcast_ref::<T>().expect("valid error");
            err.as_response()
        }))
    }
}

/// General response error.
///
/// # Create from any error types
///
/// ```
/// use poem::{error::InternalServerError, handler, Result};
///
/// #[handler]
/// async fn index() -> Result<String> {
///     Ok(std::fs::read_to_string("example.txt").map_err(InternalServerError)?)
/// }
/// ```
///
/// # Create you own error type
///
/// ```
/// use poem::{error::ResponseError, handler, http::StatusCode, Endpoint, Request, Result};
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("my error")]
/// struct MyError;
///
/// impl ResponseError for MyError {
///     fn status(&self) -> StatusCode {
///         StatusCode::BAD_GATEWAY
///     }
/// }
///
/// fn do_something() -> Result<(), MyError> {
///     Err(MyError)
/// }
///
/// #[handler]
/// async fn index() -> Result<()> {
///     Ok(do_something()?)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = index.get_response(Request::default()).await;
/// assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "my error");
/// # });
/// ```
///
/// # Custom error response
///
/// ```
/// use poem::{error::ResponseError, handler, http::StatusCode, Response, Result, Request, Body, Endpoint};
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("my error")]
/// struct MyError;
///
/// impl ResponseError for MyError {
///     fn status(&self) -> StatusCode {
///         StatusCode::BAD_GATEWAY
///     }
///
///     fn as_response(&self) -> Response {
///         let body = Body::from_json(serde_json::json!({
///             "code": 1000,
///             "message": self.to_string(),
///         })).unwrap();
///         Response::builder().status(self.status()).body(body)
///     }
/// }
///
/// fn do_something() -> Result<(), MyError> {
///     Err(MyError)
/// }
///
/// #[handler]
/// async fn index() -> Result<()> {
///     Ok(do_something()?)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = index.get_response(Request::default()).await;
/// assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
/// assert_eq!(resp.into_body().into_json::<serde_json::Value>().await.unwrap(),
/// serde_json::json!({
///     "code": 1000,
///     "message": "my error",
/// }));
/// # });
/// ```
///
/// # Downcast the error to concrete error type
/// ```
/// use poem::{error::NotFoundError, Error};
///
/// let err: Error = NotFoundError.into();
///
/// assert!(err.is::<NotFoundError>());
/// assert_eq!(err.downcast_ref::<NotFoundError>(), Some(&NotFoundError));
/// ```
pub struct Error {
    as_response: AsResponse,
    source: Option<ErrorSource>,
    extensions: Extensions,
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("source", &self.source)
            .finish()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(ErrorSource::BoxedError(err)) => Display::fmt(err, f),
            #[cfg(feature = "anyhow")]
            Some(ErrorSource::Anyhow(err)) => Display::fmt(err, f),
            #[cfg(feature = "eyre06")]
            Some(ErrorSource::Eyre06(err)) => Display::fmt(err, f),
            None => f.write_str("response"),
        }
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl<T: ResponseError + StdError + Send + Sync + 'static> From<T> for Error {
    fn from(err: T) -> Self {
        Error {
            as_response: AsResponse::from_type::<T>(),
            source: Some(ErrorSource::BoxedError(Box::new(err))),
            extensions: Extensions::default(),
        }
    }
}

impl From<Box<dyn StdError + Send + Sync>> for Error {
    fn from(err: Box<dyn StdError + Send + Sync>) -> Self {
        (StatusCode::INTERNAL_SERVER_ERROR, err).into()
    }
}

impl From<(StatusCode, Box<dyn StdError + Send + Sync>)> for Error {
    fn from((status, err): (StatusCode, Box<dyn StdError + Send + Sync>)) -> Self {
        Error {
            as_response: AsResponse::from_status(status),
            source: Some(ErrorSource::BoxedError(err)),
            extensions: Extensions::default(),
        }
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error {
            as_response: AsResponse::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            source: Some(ErrorSource::Anyhow(err)),
            extensions: Extensions::default(),
        }
    }
}

#[cfg(feature = "eyre06")]
impl From<eyre06::Error> for Error {
    fn from(err: eyre06::Error) -> Self {
        Error {
            as_response: AsResponse::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            source: Some(ErrorSource::Eyre06(err)),
            extensions: Extensions::default(),
        }
    }
}

#[cfg(feature = "anyhow")]
impl From<(StatusCode, anyhow::Error)> for Error {
    fn from((status, err): (StatusCode, anyhow::Error)) -> Self {
        Error {
            as_response: AsResponse::from_status(status),
            source: Some(ErrorSource::Anyhow(err)),
            extensions: Extensions::default(),
        }
    }
}

#[cfg(feature = "eyre06")]
impl From<(StatusCode, eyre06::Report)> for Error {
    fn from((status, err): (StatusCode, eyre06::Report)) -> Self {
        Error {
            as_response: AsResponse::from_status(status),
            source: Some(ErrorSource::Eyre06(err)),
            extensions: Extensions::default(),
        }
    }
}

impl From<StatusCode> for Error {
    fn from(status: StatusCode) -> Self {
        Error::from_status(status)
    }
}

impl Error {
    /// Create a new error object from any error type with a status code.
    #[inline]
    pub fn new<T: StdError + Send + Sync + 'static>(err: T, status: StatusCode) -> Self {
        Self {
            as_response: AsResponse::from_status(status),
            source: Some(ErrorSource::BoxedError(Box::new(err))),
            extensions: Extensions::default(),
        }
    }

    /// Create a new error object from response.
    pub fn from_response(resp: Response) -> Self {
        Self {
            as_response: AsResponse::Response(resp),
            source: None,
            extensions: Extensions::default(),
        }
    }

    /// create a new error object from status code.
    pub fn from_status(status: StatusCode) -> Self {
        #[derive(Debug, thiserror::Error)]
        #[error("{0}")]
        struct StatusError(StatusCode);

        impl ResponseError for StatusError {
            fn status(&self) -> StatusCode {
                self.0
            }

            fn as_response(&self) -> Response
            where
                Self: StdError + Send + Sync + 'static,
            {
                self.0.into_response()
            }
        }

        StatusError(status).into()
    }

    /// Create a new error object from a string with a status code.
    pub fn from_string(msg: impl Into<String>, status: StatusCode) -> Self {
        #[derive(Debug, thiserror::Error)]
        #[error("{0}")]
        struct StringError(String);

        Self::new(StringError(msg.into()), status)
    }

    /// Downcast this error object by reference.
    #[inline]
    pub fn downcast_ref<T: StdError + Send + Sync + 'static>(&self) -> Option<&T> {
        match &self.source {
            Some(ErrorSource::BoxedError(err)) => err.downcast_ref::<T>(),
            #[cfg(feature = "anyhow")]
            Some(ErrorSource::Anyhow(err)) => err.downcast_ref::<T>(),
            #[cfg(feature = "eyre06")]
            Some(ErrorSource::Eyre06(err)) => err.downcast_ref::<T>(),
            None => None,
        }
    }

    /// Attempts to downcast the error to a concrete error type.
    #[inline]
    pub fn downcast<T: StdError + Send + Sync + 'static>(self) -> Result<T, Error> {
        let as_response = self.as_response;
        let extensions = self.extensions;

        match self.source {
            Some(ErrorSource::BoxedError(err)) => match err.downcast::<T>() {
                Ok(err) => Ok(*err),
                Err(err) => Err(Error {
                    as_response,
                    source: Some(ErrorSource::BoxedError(err)),
                    extensions,
                }),
            },
            #[cfg(feature = "anyhow")]
            Some(ErrorSource::Anyhow(err)) => match err.downcast::<T>() {
                Ok(err) => Ok(err),
                Err(err) => Err(Error {
                    as_response,
                    source: Some(ErrorSource::Anyhow(err)),
                    extensions,
                }),
            },
            #[cfg(feature = "eyre06")]
            Some(ErrorSource::Eyre06(err)) => match err.downcast::<T>() {
                Ok(err) => Ok(err),
                Err(err) => Err(Error {
                    as_response,
                    source: Some(ErrorSource::Eyre06(err)),
                    extensions,
                }),
            },
            None => Err(Error {
                as_response,
                source: None,
                extensions,
            }),
        }
    }

    /// Returns `true` if the error type is the same as `T`.
    #[inline]
    pub fn is<T: StdError + Debug + Send + Sync + 'static>(&self) -> bool {
        match &self.source {
            Some(ErrorSource::BoxedError(err)) => err.is::<T>(),
            #[cfg(feature = "anyhow")]
            Some(ErrorSource::Anyhow(err)) => err.is::<T>(),
            #[cfg(feature = "eyre06")]
            Some(ErrorSource::Eyre06(err)) => err.is::<T>(),
            None => false,
        }
    }

    /// Consumes this to return a response object.
    pub fn into_response(self) -> Response {
        let mut resp = match self.as_response {
            AsResponse::Status(status) => Response::builder().status(status).body(self.to_string()),
            AsResponse::Fn(ref f) => f(&self),
            AsResponse::Response(resp) => resp,
        };
        *resp.extensions_mut() = self.extensions;
        resp
    }

    /// Returns whether the error has a source or not.
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Inserts a value to extensions
    ///
    /// Passed to `Response::extensions` when this error converted to
    /// [`Response`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use poem::{http::StatusCode, Error};
    /// let mut err = Error::from_status(StatusCode::BAD_REQUEST);
    /// err.set_data(100i32);
    ///
    /// let resp = err.into_response();
    /// assert_eq!(resp.data::<i32>(), Some(&100));
    /// ```
    #[inline]
    pub fn set_data(&mut self, data: impl Send + Sync + 'static) {
        self.extensions.insert(data);
    }

    /// Get a reference from extensions
    pub fn data<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get()
    }
}

define_http_error!(
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_REQUEST`].
    (BadRequest, BAD_REQUEST);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAUTHORIZED`].
    (Unauthorized, UNAUTHORIZED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYMENT_REQUIRED`].
    (PaymentRequired, PAYMENT_REQUIRED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::FORBIDDEN`].
    (Forbidden, FORBIDDEN);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_FOUND`].
    (NotFound, NOT_FOUND);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::METHOD_NOT_ALLOWED`].
    (MethodNotAllowed, METHOD_NOT_ALLOWED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_ACCEPTABLE`].
    (NotAcceptable, NOT_ACCEPTABLE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::PROXY_AUTHENTICATION_REQUIRED`].
    (ProxyAuthenticationRequired, PROXY_AUTHENTICATION_REQUIRED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_TIMEOUT`].
    (RequestTimeout, REQUEST_TIMEOUT);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::CONFLICT`].
    (Conflict, CONFLICT);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::GONE`].
    (Gone, GONE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::LENGTH_REQUIRED`].
    (LengthRequired, LENGTH_REQUIRED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYLOAD_TOO_LARGE`].
    (PayloadTooLarge, PAYLOAD_TOO_LARGE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::URI_TOO_LONG`].
    (UriTooLong, URI_TOO_LONG);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNSUPPORTED_MEDIA_TYPE`].
    (UnsupportedMediaType, UNSUPPORTED_MEDIA_TYPE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::RANGE_NOT_SATISFIABLE`].
    (RangeNotSatisfiable, RANGE_NOT_SATISFIABLE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::IM_A_TEAPOT`].
    (ImATeapot, IM_A_TEAPOT);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::MISDIRECTED_REQUEST`].
    (MisdirectedRequest, MISDIRECTED_REQUEST);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNPROCESSABLE_ENTITY`].
    (UnprocessableEntity, UNPROCESSABLE_ENTITY);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOCKED`].
    (Locked, LOCKED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::FAILED_DEPENDENCY`].
    (FailedDependency, FAILED_DEPENDENCY);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::UPGRADE_REQUIRED`].
    (UpgradeRequired, UPGRADE_REQUIRED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_FAILED`].
    (PreconditionFailed, PRECONDITION_FAILED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_REQUIRED`].
    (PreconditionRequired, PRECONDITION_REQUIRED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::TOO_MANY_REQUESTS`].
    (TooManyRequests, TOO_MANY_REQUESTS);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE`].
    (RequestHeaderFieldsTooLarge, REQUEST_HEADER_FIELDS_TOO_LARGE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS`].
    (UnavailableForLegalReasons, UNAVAILABLE_FOR_LEGAL_REASONS);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::EXPECTATION_FAILED`].
    (ExpectationFailed, EXPECTATION_FAILED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::INTERNAL_SERVER_ERROR`].
    (InternalServerError, INTERNAL_SERVER_ERROR);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_IMPLEMENTED`].
    (NotImplemented, NOT_IMPLEMENTED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_GATEWAY`].
    (BadGateway, BAD_GATEWAY);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::SERVICE_UNAVAILABLE`].
    (ServiceUnavailable, SERVICE_UNAVAILABLE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::GATEWAY_TIMEOUT`].
    (GatewayTimeout, GATEWAY_TIMEOUT);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::HTTP_VERSION_NOT_SUPPORTED`].
    (HttpVersionNotSupported, HTTP_VERSION_NOT_SUPPORTED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::VARIANT_ALSO_NEGOTIATES`].
    (VariantAlsoNegotiates, VARIANT_ALSO_NEGOTIATES);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::INSUFFICIENT_STORAGE`].
    (InsufficientStorage, INSUFFICIENT_STORAGE);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOOP_DETECTED`].
    (LoopDetected, LOOP_DETECTED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_EXTENDED`].
    (NotExtended, NOT_EXTENDED);
    /// Wraps any error into [`Error`] and the status code is [`StatusCode::NETWORK_AUTHENTICATION_REQUIRED`].
    (NetworkAuthenticationRequired, NETWORK_AUTHENTICATION_REQUIRED);
);

/// A specialized Result type for Poem.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;

/// Represents a type that can be converted to `poem::Result<T>`.
///
/// # Example
///
/// ```
/// use poem::error::{IntoResult, NotFoundError};
///
/// let res = "abc".into_result();
/// assert!(matches!(res, Ok("abc")));
///
/// let res = Err::<(), _>(NotFoundError).into_result();
/// assert!(res.is_err());
/// let err = res.unwrap_err();
/// assert!(err.is::<NotFoundError>());
/// ```
pub trait IntoResult<T: IntoResponse> {
    /// Consumes this value returns a `poem::Result<T>`.
    fn into_result(self) -> Result<T>;
}

impl<T, E> IntoResult<T> for Result<T, E>
where
    T: IntoResponse,
    E: Into<Error> + Send + Sync + 'static,
{
    #[inline]
    fn into_result(self) -> Result<T> {
        self.map_err(Into::into)
    }
}

impl<T: IntoResponse> IntoResult<T> for T {
    #[inline]
    fn into_result(self) -> Result<T> {
        Ok(self)
    }
}

macro_rules! define_simple_errors {
    ($($(#[$docs:meta])* ($name:ident, $status:ident, $err_msg:literal);)*) => {
        $(
        $(#[$docs])*
        #[derive(Debug, thiserror::Error, Copy, Clone, Eq, PartialEq)]
        #[error($err_msg)]
        pub struct $name;

        impl ResponseError for $name {
            fn status(&self) -> StatusCode {
                StatusCode::$status
            }
        }
        )*
    };
}

define_simple_errors!(
    /// Only the endpoints under the router can get the path parameters, otherwise this error will occur.
    (ParsePathError, BAD_REQUEST, "invalid path params");

    /// Error occurred in the router.
    (NotFoundError, NOT_FOUND, "not found");

    /// Error occurred in the router.
    (MethodNotAllowedError, METHOD_NOT_ALLOWED, "method not allowed");
);

/// A possible error value when reading the body.
#[derive(Debug, thiserror::Error)]
pub enum ReadBodyError {
    /// Body has been taken by other extractors.
    #[error("the body has been taken")]
    BodyHasBeenTaken,

    /// Body is not a valid utf8 string.
    #[error("parse utf8: {0}")]
    Utf8(#[from] FromUtf8Error),

    /// Payload too large
    #[error("payload too large")]
    PayloadTooLarge,

    /// Io error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl ResponseError for ReadBodyError {
    fn status(&self) -> StatusCode {
        match self {
            ReadBodyError::BodyHasBeenTaken => StatusCode::INTERNAL_SERVER_ERROR,
            ReadBodyError::Utf8(_) => StatusCode::BAD_REQUEST,
            ReadBodyError::Io(_) => StatusCode::BAD_REQUEST,
            ReadBodyError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }
}

/// A possible error value when parsing cookie.
#[cfg(feature = "cookie")]
#[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
#[derive(Debug, thiserror::Error)]
pub enum ParseCookieError {
    /// Cookie value is illegal.
    #[error("cookie is illegal")]
    CookieIllegal,

    /// A `Cookie` header is required.
    #[error("`Cookie` header is required")]
    CookieHeaderRequired,

    /// Cookie value is illegal.
    #[error("cookie is illegal: {0}")]
    ParseJsonValue(#[from] serde_json::Error),
}

#[cfg(feature = "cookie")]
impl ResponseError for ParseCookieError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// A possible error value when extracts data from request fails.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error("data of type `{0}` was not found.")]
pub struct GetDataError(pub &'static str);

impl ResponseError for GetDataError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// A possible error value when parsing form.
#[derive(Debug, thiserror::Error)]
pub enum ParseFormError {
    /// Invalid content type.
    #[error("invalid content type `{0}`, expect: `application/x-www-form-urlencoded`")]
    InvalidContentType(String),

    /// `Content-Type` header is required.
    #[error("expect content type `application/x-www-form-urlencoded`")]
    ContentTypeRequired,

    /// Url decode error.
    #[error("url decode: {0}")]
    UrlDecode(#[from] serde_urlencoded::de::Error),
}

impl ResponseError for ParseFormError {
    fn status(&self) -> StatusCode {
        match self {
            ParseFormError::InvalidContentType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ParseFormError::ContentTypeRequired => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ParseFormError::UrlDecode(_) => StatusCode::BAD_REQUEST,
        }
    }
}

/// A possible error value when parsing JSON.
#[derive(Debug, thiserror::Error)]
#[error("parse: {0}")]
pub struct ParseJsonError(#[from] pub serde_json::Error);

impl ResponseError for ParseJsonError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// A missing json Content-Type error value when parsing header.
#[derive(Debug, thiserror::Error)]
#[error("Missing `Content-Type: application/json`")]
pub struct MissingJsonContentTypeError;

impl ResponseError for MissingJsonContentTypeError {
    fn status(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }
}

/// A possible error value when parsing XML.
#[cfg(feature = "xml")]
#[derive(Debug, thiserror::Error)]
#[error("parse: {0}")]
pub struct ParseXmlError(#[from] pub quick_xml::de::DeError);

#[cfg(feature = "xml")]
impl ResponseError for ParseXmlError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// A missing xml Content-Type error value when parsing header.
#[cfg(feature = "xml")]
#[derive(Debug, thiserror::Error)]
#[error("Missing `Content-Type: application/xml`")]
pub struct MissingXmlContentTypeError;

#[cfg(feature = "xml")]
impl ResponseError for MissingXmlContentTypeError {
    fn status(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }
}

/// A possible error value when parsing query.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseQueryError(#[from] pub serde_urlencoded::de::Error);

impl ResponseError for ParseQueryError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// A possible error value when parsing multipart.
#[cfg(feature = "multipart")]
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
#[derive(Debug, thiserror::Error)]
pub enum ParseMultipartError {
    /// Invalid content type.
    #[error("invalid content type `{0}`, expect: `multipart/form-data`")]
    InvalidContentType(String),

    /// `Content-Type` header is required.
    #[error("expect content type `multipart/form-data`")]
    ContentTypeRequired,

    /// Parse error.
    #[error("parse: {0}")]
    Multipart(#[from] multer::Error),

    /// Body is not a valid utf8 string.
    #[error("parse utf8: {0}")]
    Utf8(#[from] FromUtf8Error),

    /// Io error
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(feature = "multipart")]
impl ResponseError for ParseMultipartError {
    fn status(&self) -> StatusCode {
        match self {
            ParseMultipartError::InvalidContentType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ParseMultipartError::ContentTypeRequired => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ParseMultipartError::Multipart(_) => StatusCode::BAD_REQUEST,
            ParseMultipartError::Utf8(_) => StatusCode::BAD_REQUEST,
            ParseMultipartError::Io(_) => StatusCode::BAD_REQUEST,
        }
    }
}

/// A possible error value when parsing typed headers.
#[derive(Debug, thiserror::Error)]
pub enum ParseTypedHeaderError {
    /// A specified header is required.
    #[error("header `{0}` is required")]
    HeaderRequired(String),

    /// Parse error.
    #[error("parse: {0}")]
    TypedHeader(#[from] headers::Error),
}

impl ResponseError for ParseTypedHeaderError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// A possible error value when handling websocket.
#[cfg(feature = "websocket")]
#[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    /// Invalid protocol
    #[error("invalid protocol")]
    InvalidProtocol,

    /// Upgrade Error
    #[error(transparent)]
    UpgradeError(#[from] UpgradeError),
}

#[cfg(feature = "websocket")]
impl ResponseError for WebSocketError {
    fn status(&self) -> StatusCode {
        match self {
            WebSocketError::InvalidProtocol => StatusCode::BAD_REQUEST,
            WebSocketError::UpgradeError(err) => err.status(),
        }
    }
}

/// A possible error value when upgrading connection.
#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    /// No upgrade
    #[error("no upgrade")]
    NoUpgrade,

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl ResponseError for UpgradeError {
    fn status(&self) -> StatusCode {
        match self {
            UpgradeError::NoUpgrade => StatusCode::INTERNAL_SERVER_ERROR,
            UpgradeError::Other(_) => StatusCode::BAD_REQUEST,
        }
    }
}

/// A possible error value when processing static files.
#[derive(Debug, thiserror::Error)]
pub enum StaticFileError {
    /// Method not allow
    #[error("method not found")]
    MethodNotAllowed(Method),

    /// Invalid path
    #[error("invalid path")]
    InvalidPath,

    /// Forbidden
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// File not found
    #[error("not found")]
    NotFound,

    /// Precondition failed
    #[error("precondition failed")]
    PreconditionFailed,

    /// Range not satisfiable
    #[error("range not satisfiable")]
    RangeNotSatisfiable {
        /// Content length
        size: u64,
    },

    /// Io error
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl ResponseError for StaticFileError {
    fn status(&self) -> StatusCode {
        match self {
            StaticFileError::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            StaticFileError::InvalidPath => StatusCode::BAD_REQUEST,
            StaticFileError::Forbidden(_) => StatusCode::FORBIDDEN,
            StaticFileError::NotFound => StatusCode::NOT_FOUND,
            StaticFileError::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
            StaticFileError::RangeNotSatisfiable { .. } => StatusCode::RANGE_NOT_SATISFIABLE,
            StaticFileError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn as_response(&self) -> Response {
        let mut resp = Response::builder()
            .status(self.status())
            .body(self.to_string());
        if let StaticFileError::RangeNotSatisfiable { size } = self {
            resp.headers_mut()
                .typed_insert(ContentRange::unsatisfied_bytes(*size));
        }
        resp
    }
}

/// A possible error value occurred in the `SizeLimit` middleware.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum SizedLimitError {
    /// Missing `Content-Length` header
    #[error("missing `Content-Length` header")]
    MissingContentLength,

    /// Payload too large
    #[error("payload too large")]
    PayloadTooLarge,
}

impl ResponseError for SizedLimitError {
    fn status(&self) -> StatusCode {
        match self {
            SizedLimitError::MissingContentLength => StatusCode::LENGTH_REQUIRED,
            SizedLimitError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }
}

/// A possible error value occurred when adding a route.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum RouteError {
    /// Invalid path
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Duplicate path
    #[error("duplicate path: {0}")]
    Duplicate(String),

    /// Invalid regex in path
    #[error("invalid regex in path: {path}")]
    InvalidRegex {
        /// Path
        path: String,

        /// Regex
        regex: String,
    },
}

impl ResponseError for RouteError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// A possible error value occurred in the `Cors` middleware.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CorsError {
    /// Method not allowed
    #[error("request-method not allowed")]
    MethodNotAllowed,

    /// Origin not allowed
    #[error("request-origin not allowed")]
    OriginNotAllowed,

    /// Headers not allowed
    #[error("request-headers not allowed")]
    HeadersNotAllowed,
}

impl ResponseError for CorsError {
    fn status(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}

/// A possible error value occurred when loading i18n resources.
#[cfg(feature = "i18n")]
#[derive(Debug, thiserror::Error)]
pub enum I18NError {
    /// Fluent error.
    #[error("fluent: {}", .0[0])]
    Fluent(Vec<fluent::FluentError>),

    /// Fluent FTL parser error.
    #[error("fluent parser: {}", .0[0])]
    FluentParser(Vec<fluent_syntax::parser::ParserError>),

    /// There is no value in the message.
    #[error("no value")]
    FluentNoValue,

    /// Message id was not found.
    #[error("msg not found: `{id}`")]
    FluentMessageNotFound {
        /// Message id
        id: String,
    },

    /// Invalid language id.
    #[error("invalid language id: {0}")]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),

    /// Io error
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(feature = "i18n")]
impl ResponseError for I18NError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error as IoError, ErrorKind};

    use super::*;

    #[test]
    fn test_into_result() {
        assert!(matches!("hello".into_result(), Ok("hello")));
        assert!(matches!(Ok::<_, Error>("hello").into_result(), Ok("hello")));
        assert!(matches!(
            Ok::<_, NotFoundError>("hello").into_result(),
            Ok("hello")
        ));
        assert!(Err::<String, _>(NotFoundError)
            .into_result()
            .unwrap_err()
            .is::<NotFoundError>());
    }

    #[test]
    fn test_error() {
        let err = Error::new(
            IoError::new(ErrorKind::AlreadyExists, "aaa"),
            StatusCode::BAD_GATEWAY,
        );
        assert!(err.is::<IoError>());
        assert_eq!(
            err.downcast_ref::<IoError>().unwrap().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(err.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_box_error() {
        let boxed_err: Box<dyn StdError + Send + Sync> =
            Box::new(IoError::new(ErrorKind::AlreadyExists, "aaa"));
        let err: Error = Error::from((StatusCode::BAD_GATEWAY, boxed_err));
        assert!(err.is::<IoError>());
        assert_eq!(
            err.downcast_ref::<IoError>().unwrap().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(err.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[cfg(feature = "anyhow")]
    #[test]
    fn test_anyhow_error() {
        let anyhow_err: anyhow::Error = IoError::new(ErrorKind::AlreadyExists, "aaa").into();
        let err: Error = Error::from((StatusCode::BAD_GATEWAY, anyhow_err));
        assert!(err.is::<IoError>());
        assert_eq!(
            err.downcast_ref::<IoError>().unwrap().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(err.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[cfg(feature = "eyre6")]
    #[test]
    fn test_eyre6_error() {
        let eyre6_err: eyre6::Error = IoError::new(ErrorKind::AlreadyExists, "aaa").into();
        let err: Error = Error::from((StatusCode::BAD_GATEWAY, eyre6_err));
        assert!(err.is::<IoError>());
        assert_eq!(
            err.downcast_ref::<IoError>().unwrap().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(err.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_custom_as_response() {
        #[derive(Debug, thiserror::Error)]
        #[error("my error")]
        struct MyError;

        impl ResponseError for MyError {
            fn status(&self) -> StatusCode {
                StatusCode::BAD_GATEWAY
            }

            fn as_response(&self) -> Response {
                Response::builder()
                    .status(self.status())
                    .body("my error message")
            }
        }

        let err = Error::from(MyError);
        let resp = err.into_response();

        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            resp.into_body().into_string().await.unwrap(),
            "my error message"
        );
    }
}
