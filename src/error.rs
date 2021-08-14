//! Some common error types.

use std::{
    convert::Infallible,
    fmt::{self, Debug, Display, Formatter},
};

pub use crate::http::{
    header::{
        InvalidHeaderName as ErrorInvalidHeaderName, InvalidHeaderValue as ErrorInvalidHeaderValue,
    },
    method::InvalidMethod as ErrorInvalidMethod,
    status::InvalidStatusCode as ErrorInvalidStatusCode,
    uri::{InvalidUri as ErrorInvalidUri, InvalidUriParts as ErrorInvalidUriParts},
};
use crate::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};

macro_rules! define_error {
    ($($(#[$docs:meta])* ($name:ident, $code:ident);)*) => {
        $(
        $(#[$docs])*
        #[inline]
        pub fn $name(error: impl Into<anyhow::Error>) -> Self {
            Self {
                status: StatusCode::$code,
                error: error.into(),
            }
        }
        )*
    };
}

/// General error.
///
/// In Poem, almost all functions that may return errors return this type, so
/// you don't need to perform tedious error type conversion.
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    error: anyhow::Error,
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl Error {
    /// Create a new error from any error.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::ParseIntError;
    ///
    /// use poem::prelude::*;
    /// use poem::http::StatusCode;
    ///
    /// let err = Error::new(StatusCode::BAD_REQUEST, "a".parse::<i32>().unwrap_err());
    /// assert!(err.downcast_ref::<ParseIntError>().is_some());
    /// ```
    #[inline]
    pub fn new(status: StatusCode, error: impl Into<anyhow::Error>) -> Self {
        Self {
            status,
            error: error.into(),
        }
    }

    /// Attempts to downcast the error to a concrete error type.
    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Display + Debug + Send + Sync + 'static,
    {
        self.error.downcast_ref::<T>()
    }

    /// Returns true if the concrete error type is the same as T.
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: Display + Debug + Send + Sync + 'static,
    {
        self.error.is::<T>()
    }

    /// Returns true if the concrete error type is [`ErrorNotFound`].
    #[inline]
    pub fn is_not_found(&self) -> bool {
        self.is::<ErrorNotFound>()
    }

    pub(crate) fn as_response(&self) -> Response {
        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from_string(self.error.to_string()))
            .unwrap()
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

macro_rules! define_simple_errors {
    ($($(#[$docs:meta])* ($name:ident, $status:ident, $err_msg:literal);)*) => {
        $(
        $(#[$docs])*
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct $name;

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", $err_msg)
            }
        }

        impl std::error::Error for $name {}

        impl From<$name> for Error {
            fn from(err: $name) -> Error {
                Error::new(StatusCode::$status, err)
            }
        }
        )*
    };
}

define_simple_errors!(
    /// This error occurs when the path does not match.
    (ErrorNotFound, BAD_REQUEST, "not found");

    /// This error occurs when the status code is invalid.
    (ErrorMissingRouteParams, INTERNAL_SERVER_ERROR, "missing route params");

    /// Only the endpoints under the router can get the path parameters, otherwise this error will occur.
    (ErrorInvalidPathParams, INTERNAL_SERVER_ERROR, "invalid path params");

    /// This error occurs when `Content-type` is not `application/x-www-form-urlencoded`.
    (ErrorInvalidFormContentType, BAD_REQUEST, "invalid form content type");
);

impl From<ErrorInvalidUri> for Error {
    fn from(err: ErrorInvalidUri) -> Self {
        Error::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

impl From<ErrorInvalidUriParts> for Error {
    fn from(err: ErrorInvalidUriParts) -> Self {
        Error::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

// impl From<ErrorInvalidHeaderName> for Error {
//     fn from(err: ErrorInvalidHeaderName) -> Self {
//         Error::new(StatusCode::INTERNAL_SERVER_ERROR, err)
//     }
// }

impl From<ErrorInvalidHeaderValue> for Error {
    fn from(err: ErrorInvalidHeaderValue) -> Self {
        Error::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

impl From<ErrorInvalidMethod> for Error {
    fn from(err: ErrorInvalidMethod) -> Self {
        Error::new(StatusCode::BAD_REQUEST, err)
    }
}

impl From<ErrorInvalidStatusCode> for Error {
    fn from(err: ErrorInvalidStatusCode) -> Self {
        Error::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

/// A specialized Result type for Poem.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;
