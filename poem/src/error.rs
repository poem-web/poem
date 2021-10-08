//! Some common error types.

use std::{
    convert::Infallible,
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    string::FromUtf8Error,
};

use crate::{http::StatusCode, IntoResponse, Response};

macro_rules! define_error {
    ($($(#[$docs:meta])* ($name:ident, $status:ident);)*) => {
        $(
        $(#[$docs])*
        #[inline]
        pub fn $name(err: impl StdError + Send + Sync + 'static) -> Self {
            Self::new(StatusCode::$status).with_reason(err)
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

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.as_response()
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

    define_error!(
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_REQUEST`].
        (bad_request, BAD_REQUEST);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAUTHORIZED`].
        (unauthorized, UNAUTHORIZED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYMENT_REQUIRED`].
        (payment_required, PAYMENT_REQUIRED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::FORBIDDEN`].
        (forbidden, FORBIDDEN);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_FOUND`].
        (not_found, NOT_FOUND);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::METHOD_NOT_ALLOWED`].
        (method_not_allowed, METHOD_NOT_ALLOWED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_ACCEPTABLE`].
        (not_acceptable, NOT_ACCEPTABLE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PROXY_AUTHENTICATION_REQUIRED`].
        (proxy_authentication_required, PROXY_AUTHENTICATION_REQUIRED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_TIMEOUT`].
        (request_timeout, REQUEST_TIMEOUT);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::CONFLICT`].
        (conflict, CONFLICT);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::GONE`].
        (gone, GONE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LENGTH_REQUIRED`].
        (length_required, LENGTH_REQUIRED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYLOAD_TOO_LARGE`].
        (payload_too_large, PAYLOAD_TOO_LARGE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::URI_TOO_LONG`].
        (uri_too_long, URI_TOO_LONG);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNSUPPORTED_MEDIA_TYPE`].
        (unsupported_media_type, UNSUPPORTED_MEDIA_TYPE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::RANGE_NOT_SATISFIABLE`].
        (range_not_satisfiable, RANGE_NOT_SATISFIABLE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::IM_A_TEAPOT`].
        (im_a_teapot, IM_A_TEAPOT);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::MISDIRECTED_REQUEST`].
        (misdirected_request, MISDIRECTED_REQUEST);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNPROCESSABLE_ENTITY`].
        (unprocessable_entity, UNPROCESSABLE_ENTITY);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOCKED`].
        (locked, LOCKED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::FAILED_DEPENDENCY`].
        (failed_dependency, FAILED_DEPENDENCY);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UPGRADE_REQUIRED`].
        (upgrade_required, UPGRADE_REQUIRED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_FAILED`].
        (precondition_failed, PRECONDITION_FAILED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_REQUIRED`].
        (precondition_required, PRECONDITION_REQUIRED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::TOO_MANY_REQUESTS`].
        (too_many_requests, TOO_MANY_REQUESTS);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE`].
        (request_header_fields_too_large, REQUEST_HEADER_FIELDS_TOO_LARGE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS`].
        (unavailable_for_legal_reasons, UNAVAILABLE_FOR_LEGAL_REASONS);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::EXPECTATION_FAILED`].
        (expectation_failed, EXPECTATION_FAILED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::INTERNAL_SERVER_ERROR`].
        (internal_server_error, INTERNAL_SERVER_ERROR);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_IMPLEMENTED`].
        (not_implemented, NOT_IMPLEMENTED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_GATEWAY`].
        (bad_gateway, BAD_GATEWAY);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::SERVICE_UNAVAILABLE`].
        (service_unavailable, SERVICE_UNAVAILABLE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::GATEWAY_TIMEOUT`].
        (gateway_timeout, GATEWAY_TIMEOUT);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::HTTP_VERSION_NOT_SUPPORTED`].
        (http_version_not_supported, HTTP_VERSION_NOT_SUPPORTED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::VARIANT_ALSO_NEGOTIATES`].
        (variant_also_negotiates, VARIANT_ALSO_NEGOTIATES);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::INSUFFICIENT_STORAGE`].
        (insufficient_storage, INSUFFICIENT_STORAGE);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOOP_DETECTED`].
        (loop_detected, LOOP_DETECTED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_EXTENDED`].
        (not_extended, NOT_EXTENDED);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NETWORK_AUTHENTICATION_REQUIRED`].
        (network_authentication_required, NETWORK_AUTHENTICATION_REQUIRED);
    );
}

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
    (ErrorInvalidPathParams, INTERNAL_SERVER_ERROR, "invalid path params");
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
            ReadBodyError::BodyHasBeenTaken => Error::internal_server_error(err),
            ReadBodyError::Utf8(err) => Error::bad_request(err),
            ReadBodyError::Io(err) => Error::bad_request(err),
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
        Error::bad_request(err)
    }
}

/// A possible error value when extracts data from request fails.
#[derive(Debug, thiserror::Error)]
#[error("data of type `{0}` was not found.")]
pub struct GetDataError(pub &'static str);

impl From<GetDataError> for Error {
    fn from(err: GetDataError) -> Self {
        Error::internal_server_error(err)
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
        Error::bad_request(err)
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
        Error::bad_request(err)
    }
}

/// A possible error value when parsing query.
#[derive(Debug, thiserror::Error)]
#[error("parse: {0}")]
pub struct ParseQueryError(#[from] pub serde_urlencoded::de::Error);

impl From<ParseQueryError> for Error {
    fn from(err: ParseQueryError) -> Self {
        Error::bad_request(err)
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
        Error::bad_request(err)
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
        Error::bad_request(err)
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
            WebSocketError::InvalidProtocol => Error::bad_request(err),
            WebSocketError::NoUpgrade => Error::internal_server_error(err),
        }
    }
}
