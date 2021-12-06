//! Some common error types.

use std::{
    fmt::{Debug, Display},
    string::FromUtf8Error,
};

use crate::{http::StatusCode, IntoResponse, Response};

macro_rules! define_http_error {
    ($($(#[$docs:meta])* ($name:ident, $status:ident);)*) => {
        $(
        $(#[$docs])*
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name(err: impl Display) -> Error {
            Error::new(StatusCode::$status).with_reason(err)
        }
        )*
    };
}

/// General response error.
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    reason: Option<String>,
}

impl<T: Display> From<T> for Error {
    #[inline]
    fn from(err: T) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            reason: Some(err.to_string()),
        }
    }
}

impl IntoResponse for Error {
    #[inline]
    fn into_response(self) -> Response {
        self.as_response()
    }
}

impl Error {
    /// Create a new error with status code.
    #[inline]
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            reason: None,
        }
    }

    /// Sets the reason for this error.
    #[inline]
    pub fn with_reason(self, reason: impl Display) -> Self {
        Self {
            reason: Some(reason.to_string()),
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
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Creates full response for this error.
    #[inline]
    pub fn as_response(&self) -> Response {
        match &self.reason {
            Some(reason) => Response::builder()
                .status(self.status)
                .body(reason.to_string()),
            None => Response::builder().status(self.status).finish(),
        }
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
            fn from(_: $name) -> Self {
                Error::new(StatusCode::$status).with_reason($err_msg)
            }
        }

        impl IntoResponse for $name {
            fn into_response(self) -> Response {
                Into::<Error>::into(self).as_response()
            }
        }

        )*
    };
}

define_simple_errors!(
    /// Only the endpoints under the router can get the path parameters, otherwise this error will occur.
    (ParsePathError, BAD_REQUEST, "invalid path params");
);

/// A possible error value when reading the body.
#[derive(Debug)]
pub enum ReadBodyError {
    /// Body has been taken by other extractors.
    BodyHasBeenTaken,

    /// Body is not a valid utf8 string.
    Utf8(FromUtf8Error),

    /// Io error.
    Io(std::io::Error),
}

impl From<FromUtf8Error> for ReadBodyError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<std::io::Error> for ReadBodyError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ReadBodyError> for Error {
    fn from(err: ReadBodyError) -> Self {
        match err {
            ReadBodyError::BodyHasBeenTaken => {
                Error::new(StatusCode::INTERNAL_SERVER_ERROR).with_reason("the body has been taken")
            }
            ReadBodyError::Utf8(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("parse utf8: {}", err))
            }
            ReadBodyError::Io(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("io: {}", err))
            }
        }
    }
}

impl IntoResponse for ReadBodyError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing cookie.
#[cfg(feature = "cookie")]
#[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
#[derive(Debug)]
pub enum ParseCookieError {
    /// Cookie value is illegal.
    CookieIllegal,

    /// A `Cookie` header is required.
    CookieHeaderRequired,

    /// Cookie value is illegal.
    ParseJsonValue(serde_json::Error),
}

#[cfg(feature = "cookie")]
impl From<ParseCookieError> for Error {
    fn from(err: ParseCookieError) -> Self {
        match err {
            ParseCookieError::CookieIllegal => {
                Error::new(StatusCode::BAD_REQUEST).with_reason("cookie is illegal")
            }
            ParseCookieError::CookieHeaderRequired => {
                Error::new(StatusCode::BAD_REQUEST).with_reason("`Cookie` header is required")
            }
            ParseCookieError::ParseJsonValue(_) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason("cookie is illegal")
            }
        }
    }
}

#[cfg(feature = "cookie")]
impl IntoResponse for ParseCookieError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when extracts data from request fails.
#[derive(Debug)]
pub struct GetDataError(pub &'static str);

impl From<GetDataError> for Error {
    fn from(err: GetDataError) -> Self {
        Error::new(StatusCode::INTERNAL_SERVER_ERROR)
            .with_reason(format!("data of type `{}` was not found.", err.0))
    }
}

impl IntoResponse for GetDataError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing form.
#[derive(Debug)]
pub enum ParseFormError {
    /// Read body error.
    ReadBody(ReadBodyError),

    /// Invalid content type.
    InvalidContentType(String),

    /// `Content-Type` header is required.
    ContentTypeRequired,

    /// Url decode error.
    UrlDecode(serde_urlencoded::de::Error),
}

impl From<ReadBodyError> for ParseFormError {
    fn from(err: ReadBodyError) -> Self {
        Self::ReadBody(err)
    }
}

impl From<serde_urlencoded::de::Error> for ParseFormError {
    fn from(err: serde_urlencoded::de::Error) -> Self {
        Self::UrlDecode(err)
    }
}

impl From<ParseFormError> for Error {
    fn from(err: ParseFormError) -> Self {
        match err {
            ParseFormError::ReadBody(err) => err.into(),
            ParseFormError::InvalidContentType(content_type) => Error::new(StatusCode::BAD_REQUEST)
                .with_reason(format!(
                    "invalid content type `{}`, expect: `application/x-www-form-urlencoded`",
                    content_type
                )),
            ParseFormError::ContentTypeRequired => Error::new(StatusCode::BAD_REQUEST)
                .with_reason("expect content type `application/x-www-form-urlencoded`"),
            ParseFormError::UrlDecode(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("url decode: {}", err))
            }
        }
    }
}

impl IntoResponse for ParseFormError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing JSON.
#[derive(Debug)]
pub enum ParseJsonError {
    /// Read body error.
    ReadBody(ReadBodyError),

    /// Parse error.
    Json(serde_json::Error),
}

impl From<ReadBodyError> for ParseJsonError {
    fn from(err: ReadBodyError) -> Self {
        Self::ReadBody(err)
    }
}

impl From<serde_json::Error> for ParseJsonError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<ParseJsonError> for Error {
    fn from(err: ParseJsonError) -> Self {
        match err {
            ParseJsonError::ReadBody(err) => err.into(),
            ParseJsonError::Json(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("parse: {}", err))
            }
        }
    }
}

impl IntoResponse for ParseJsonError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing query.
#[derive(Debug)]
pub struct ParseQueryError(pub serde_urlencoded::de::Error);

impl From<serde_urlencoded::de::Error> for ParseQueryError {
    fn from(err: serde::de::value::Error) -> Self {
        ParseQueryError(err)
    }
}

impl From<ParseQueryError> for Error {
    fn from(err: ParseQueryError) -> Self {
        Error::new(StatusCode::BAD_REQUEST).with_reason(err.0.to_string())
    }
}

impl IntoResponse for ParseQueryError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing multipart.
#[cfg(feature = "multipart")]
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
#[derive(Debug)]
pub enum ParseMultipartError {
    /// Read body error.
    ReadBody(ReadBodyError),

    /// Invalid content type.
    InvalidContentType(String),

    /// `Content-Type` header is required.
    ContentTypeRequired,

    /// Parse error.
    Multipart(multer::Error),
}

#[cfg(feature = "multipart")]
impl From<ReadBodyError> for ParseMultipartError {
    fn from(err: ReadBodyError) -> Self {
        Self::ReadBody(err)
    }
}
#[cfg(feature = "multipart")]
impl From<multer::Error> for ParseMultipartError {
    fn from(err: multer::Error) -> Self {
        Self::Multipart(err)
    }
}

#[cfg(feature = "multipart")]
impl From<ParseMultipartError> for Error {
    fn from(err: ParseMultipartError) -> Self {
        match err {
            ParseMultipartError::ReadBody(err) => err.into(),
            ParseMultipartError::InvalidContentType(content_type) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!(
                    "invalid content type `{}`, expect: `multipart/form-data`",
                    content_type
                ))
            }
            ParseMultipartError::ContentTypeRequired => Error::new(StatusCode::BAD_REQUEST)
                .with_reason("expect content type `multipart/form-data`"),
            ParseMultipartError::Multipart(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("parse: {}", err))
            }
        }
    }
}

#[cfg(feature = "multipart")]
impl IntoResponse for ParseMultipartError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when parsing typed headers.
#[derive(Debug)]
pub enum ParseTypedHeaderError {
    /// A specified header is required.
    HeaderRequired(String),

    /// Parse error.
    TypedHeader(headers::Error),
}

impl From<headers::Error> for ParseTypedHeaderError {
    fn from(err: headers::Error) -> Self {
        Self::TypedHeader(err)
    }
}

impl From<ParseTypedHeaderError> for Error {
    fn from(err: ParseTypedHeaderError) -> Self {
        match err {
            ParseTypedHeaderError::HeaderRequired(header_name) => {
                Error::new(StatusCode::BAD_REQUEST)
                    .with_reason(format!("header `{}` is required", header_name))
            }
            ParseTypedHeaderError::TypedHeader(err) => {
                Error::new(StatusCode::BAD_REQUEST).with_reason(format!("parse: {}", err))
            }
        }
    }
}

impl IntoResponse for ParseTypedHeaderError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when handling websocket.
#[cfg(feature = "websocket")]
#[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
#[derive(Debug)]
pub enum WebSocketError {
    /// Invalid protocol
    InvalidProtocol,

    /// Upgrade Error
    UpgradeError(UpgradeError),
}

#[cfg(feature = "websocket")]
impl From<UpgradeError> for WebSocketError {
    fn from(err: UpgradeError) -> Self {
        Self::UpgradeError(err)
    }
}

#[cfg(feature = "websocket")]
impl From<WebSocketError> for Error {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::InvalidProtocol => {
                Error::new(StatusCode::BAD_REQUEST).with_reason("invalid protocol")
            }
            WebSocketError::UpgradeError(err) => err.into(),
        }
    }
}

#[cfg(feature = "websocket")]
impl IntoResponse for WebSocketError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

/// A possible error value when upgrading connection.
#[derive(Debug)]
pub enum UpgradeError {
    /// No upgrade
    NoUpgrade,

    /// Other error
    Other(String),
}

impl From<UpgradeError> for Error {
    fn from(err: UpgradeError) -> Self {
        match err {
            UpgradeError::NoUpgrade => {
                Error::new(StatusCode::INTERNAL_SERVER_ERROR).with_reason("no upgrade")
            }
            UpgradeError::Other(err) => Error::new(StatusCode::BAD_REQUEST).with_reason(err),
        }
    }
}

impl IntoResponse for UpgradeError {
    fn into_response(self) -> Response {
        Into::<Error>::into(self).as_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_into_error() {
        let err: Error = "a".into();
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.reason(), Some("a"));
    }

    #[test]
    fn extractor_err_into_error() {
        let err: Error = ReadBodyError::BodyHasBeenTaken.into();
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err: Error = ParseFormError::ContentTypeRequired.into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }
}
