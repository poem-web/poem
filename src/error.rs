//! Some common error types.

use std::fmt::{self, Debug, Display, Formatter};

use crate::{http::StatusCode, IntoResponse, Response};

macro_rules! define_error {
    ($($(#[$docs:meta])* ($name:ident, $status:ident);)*) => {
        $(
        $(#[$docs])*
        #[inline]
        pub fn $name(err: impl Display) -> Self {
            Self::new(StatusCode::$status, err)
        }
        )*
    };
}

/// General error.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Error {
    status: StatusCode,
    reason: Option<String>,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.as_response()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            self.status.as_u16(),
            self.reason
                .as_deref()
                .or_else(|| self.status.canonical_reason())
                .unwrap_or("unknown")
        )
    }
}

impl Error {
    /// Create a new error
    #[inline]
    pub fn new(status: StatusCode, reason: impl Display) -> Self {
        Self {
            status,
            reason: Some(reason.to_string()),
        }
    }

    /// Create a new error with status code.
    #[inline]
    pub fn status(status: StatusCode) -> Self {
        Self {
            status,
            reason: None,
        }
    }

    /// Creates full response for this error.
    #[inline]
    pub fn as_response(&self) -> Response {
        Response::builder()
            .status(self.status)
            .body(self.to_string())
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
