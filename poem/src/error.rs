//! Some common error types.

use std::{
    convert::Infallible,
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    string::FromUtf8Error,
};

use crate::{http::StatusCode, IntoResponse, Response};

macro_rules! define_http_error {
    ($($(#[$docs:meta])* ($name:ident, $status:ident);)*) => {
        $(
        $(#[$docs])*
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name(err: impl StdError + Send + Sync + 'static) -> Error {
            Error::new(StatusCode::$status).with_reason(err)
        }
        )*
    };
}

/// General error.
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    reason: anyhow::Error,
}

impl<T: Into<Error> + Send> IntoResponse for T {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.status.as_u16(), self.reason())
    }
}

#[derive(Debug)]
struct StatusError(StatusCode);

impl Display for StatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for StatusError {}

impl Error {
    /// Create a new error with status code.
    #[inline]
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            reason: anyhow::Error::from(StatusError(status)),
        }
    }

    /// Sets the reason for this error.
    #[inline]
    pub fn with_reason(self, reason: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            reason: anyhow::Error::from(reason),
            ..self
        }
    }

    /// Sets the reason string for this error.
    #[inline]
    pub fn with_reason_string(self, reason: impl Display + Debug + Send + Sync + 'static) -> Self {
        Self {
            reason: anyhow::Error::msg(reason),
            ..self
        }
    }

    /// Returns the status code of this error.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the reason of this error.
    #[inline]
    pub fn reason(&self) -> &impl Display {
        &self.reason
    }

    /// Downcast this error object by reference.
    pub fn downcast_ref<T: Display + Debug + Send + Sync + 'static>(&self) -> Option<&T> {
        self.reason.downcast_ref()
    }

    /// Creates full response for this error.
    #[inline]
    pub fn as_response(&self) -> Response {
        Response::builder()
            .status(self.status)
            .body(self.reason().to_string())
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

macro_rules! define_simple_errors {
    ($($(#[$docs:meta])* ($name:ident, $status:ident, $err_msg:literal);)*) => {
        $(
        $(#[$docs])*
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct $name;

        impl From<$name> for Error {
            fn from(_: $name) -> Error {
                Error::new(StatusCode::$status).with_reason(SimpleError($err_msg))
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", $err_msg)
            }
        }

        impl std::error::Error for $name {}
        )*
    };
}

#[derive(Debug)]
struct SimpleError(&'static str);

impl Display for SimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for SimpleError {}

define_simple_errors!(
    /// Only the endpoints under the router can get the path parameters, otherwise this error will occur.
    (ErrorInvalidPathParams, BAD_REQUEST, "invalid path params");
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

    /// Io error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ReadBodyError> for Error {
    fn from(err: ReadBodyError) -> Self {
        match err {
            ReadBodyError::BodyHasBeenTaken => InternalServerError(err),
            ReadBodyError::Utf8(err) => BadRequest(err),
            ReadBodyError::Io(err) => BadRequest(err),
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
    #[error("cookie is illegal")]
    ParseJsonValue(serde_json::Error),
}

#[cfg(feature = "cookie")]
impl From<ParseCookieError> for Error {
    fn from(err: ParseCookieError) -> Self {
        BadRequest(err)
    }
}

/// A possible error value when extracts data from request fails.
#[derive(Debug, thiserror::Error)]
#[error("data of type `{0}` was not found.")]
pub struct GetDataError(pub &'static str);

impl From<GetDataError> for Error {
    fn from(err: GetDataError) -> Self {
        InternalServerError(err)
    }
}

/// A possible error value when parsing form.
#[derive(Debug, thiserror::Error)]
pub enum ParseFormError {
    /// Read body error.
    #[error("read body: {0}")]
    ReadBody(#[from] ReadBodyError),

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

impl From<ParseFormError> for Error {
    fn from(err: ParseFormError) -> Self {
        BadRequest(err)
    }
}

/// A possible error value when parsing JSON.
#[derive(Debug, thiserror::Error)]
pub enum ParseJsonError {
    /// Read body error.
    #[error("read body: {0}")]
    ReadBody(#[from] ReadBodyError),

    /// Parse error.
    #[error("parse: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<ParseJsonError> for Error {
    fn from(err: ParseJsonError) -> Self {
        BadRequest(err)
    }
}

/// A possible error value when parsing query.
#[derive(Debug, thiserror::Error)]
#[error("parse: {0}")]
pub struct ParseQueryError(#[from] pub serde_urlencoded::de::Error);

impl From<ParseQueryError> for Error {
    fn from(err: ParseQueryError) -> Self {
        BadRequest(err)
    }
}

/// A possible error value when parsing multipart.
#[cfg(feature = "multipart")]
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
#[derive(Debug, thiserror::Error)]
pub enum ParseMultipartError {
    /// Read body error.
    #[error("read body: {0}")]
    ReadBody(#[from] ReadBodyError),

    /// Invalid content type.
    #[error("invalid content type `{0}`, expect: `multipart/form-data`")]
    InvalidContentType(String),

    /// `Content-Type` header is required.
    #[error("expect content type `multipart/form-data`")]
    ContentTypeRequired,

    /// Parse error.
    #[error("parse: {0}")]
    Multipart(#[from] multer::Error),
}

#[cfg(feature = "multipart")]
impl From<ParseMultipartError> for Error {
    fn from(err: ParseMultipartError) -> Self {
        BadRequest(err)
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

impl From<ParseTypedHeaderError> for Error {
    fn from(err: ParseTypedHeaderError) -> Self {
        BadRequest(err)
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

    /// No upgrade
    #[error("no upgrade")]
    NoUpgrade,
}

#[cfg(feature = "websocket")]
impl From<WebSocketError> for Error {
    fn from(err: WebSocketError) -> Self {
        match &err {
            WebSocketError::InvalidProtocol => BadRequest(err),
            WebSocketError::NoUpgrade => InternalServerError(err),
        }
    }
}
