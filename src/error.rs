//! Some common error types.

use std::fmt::{self, Debug, Display, Formatter};

use crate::{http::StatusCode, Body, Response};

macro_rules! define_error {
    ($($(#[$docs:meta])* ($name:ident, $err:ident);)*) => {
        $(
        $(#[$docs])*
        #[inline]
        pub fn $name(err: impl Display) -> Self {
            Error::new($err::new(err))
        }
        )*
    };
}

/// Errors that can generate responses.
pub trait ResponseError: Display + Debug + Send + Sync + 'static {
    /// Creates full response for error.
    fn as_response(&self) -> Response;
}

/// General error.
pub struct Error(Box<dyn ResponseError>);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: ResponseError> From<T> for Error {
    fn from(err: T) -> Self {
        Self(Box::new(err))
    }
}

impl Error {
    /// Create a new error
    #[inline]
    pub fn new(err: impl ResponseError) -> Self {
        Self(Box::new(err))
    }

    /// Create a new error with status code.
    #[inline]
    pub fn status(status: StatusCode) -> Self {
        struct StatusError(StatusCode);

        impl Display for StatusError {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Display::fmt(&self.0, f)
            }
        }

        impl Debug for StatusError {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Debug::fmt(&self.0, f)
            }
        }

        impl ResponseError for StatusError {
            fn as_response(&self) -> Response {
                Response::builder().status(self.0).body(Body::from_string(
                    self.0
                        .canonical_reason()
                        .unwrap_or_else(|| self.0.as_str())
                        .to_string(),
                ))
            }
        }

        Self::new(StatusError(status))
    }

    /// Creates full response for error.
    #[inline]
    pub fn as_response(&self) -> Response {
        self.0.as_response()
    }

    define_error!(
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_REQUEST`].
        (bad_request, ErrorBadRequest);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAUTHORIZED`].
        (unauthorized, ErrorUnauthorized);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYMENT_REQUIRED`].
        (payment_required, ErrorPaymentRequired);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::FORBIDDEN`].
        (forbidden, ErrorForbidden);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_FOUND`].
        (not_found, ErrorNotFound);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::METHOD_NOT_ALLOWED`].
        (method_not_allowed, ErrorMethodNotAllowed);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_ACCEPTABLE`].
        (not_acceptable, ErrorNotAcceptable);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PROXY_AUTHENTICATION_REQUIRED`].
        (proxy_authentication_required, ErrorProxyAuthenticationRequired);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_TIMEOUT`].
        (request_timeout, ErrorRequestTimeout);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::CONFLICT`].
        (conflict, ErrorConflict);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::GONE`].
        (gone, ErrorGone);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LENGTH_REQUIRED`].
        (length_required, ErrorLengthRequired);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PAYLOAD_TOO_LARGE`].
        (payload_too_large, ErrorPayloadTooLarge);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::URI_TOO_LONG`].
        (uri_too_long, ErrorUriTooLong);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNSUPPORTED_MEDIA_TYPE`].
        (unsupported_media_type, ErrorUnsupportedMediaType);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::RANGE_NOT_SATISFIABLE`].
        (range_not_satisfiable, ErrorRangeNotSatisfiable);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::IM_A_TEAPOT`].
        (im_a_teapot, ErrorImATeapot);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::MISDIRECTED_REQUEST`].
        (misdirected_request, ErrorMisdirectedRequest);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNPROCESSABLE_ENTITY`].
        (unprocessable_entity, ErrorUnprocessableEntity);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOCKED`].
        (locked, ErrorLocked);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::FAILED_DEPENDENCY`].
        (failed_dependency, ErrorFailedDependency);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UPGRADE_REQUIRED`].
        (upgrade_required, ErrorUpgradeRequired);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_FAILED`].
        (precondition_failed, ErrorPreconditionFailed);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::PRECONDITION_REQUIRED`].
        (precondition_required, ErrorPreconditionRequired);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::TOO_MANY_REQUESTS`].
        (too_many_requests, ErrorTooManyRequests);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE`].
        (request_header_fields_too_large, ErrorRequestHeaderFieldsTooLarge);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS`].
        (unavailable_for_legal_reasons, ErrorUnavailableForLegalReasons);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::EXPECTATION_FAILED`].
        (expectation_failed, ErrorExpectationFailed);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::INTERNAL_SERVER_ERROR`].
        (internal_server_error, ErrorInternalServerError);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_IMPLEMENTED`].
        (not_implemented, ErrorNotImplemented);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::BAD_GATEWAY`].
        (bad_gateway, ErrorBadGateway);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::SERVICE_UNAVAILABLE`].
        (service_unavailable, ErrorServiceUnavailable);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::GATEWAY_TIMEOUT`].
        (gateway_timeout, ErrorGatewayTimeout);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::HTTP_VERSION_NOT_SUPPORTED`].
        (http_version_not_supported, ErrorHttpVersionNotSupported);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::VARIANT_ALSO_NEGOTIATES`].
        (variant_also_negotiates, ErrorVariantAlsoNegotiates);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::INSUFFICIENT_STORAGE`].
        (insufficient_storage, ErrorInsufficientStorage);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::LOOP_DETECTED`].
        (loop_detected, ErrorLoopDetected);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NOT_EXTENDED`].
        (not_extended, ErrorNotExtended);
        /// Wraps any error into [`Error`] and the status code is [`StatusCode::NETWORK_AUTHENTICATION_REQUIRED`].
        (network_authentication_required, ErrorNetworkAuthenticationRequired);
    );
}

macro_rules! define_status_error {
    ($($(#[$docs:meta])* ($name:ident, $code:ident);)*) => {
        $(
        $(#[$docs])*
        pub struct $name(String);

        impl Default for $name {
            fn default() -> Self {
                Self(StatusCode::$code.canonical_reason().unwrap().to_string())
            }
        }

        impl $name {
            /// Create an error.
            #[inline]
            pub fn new(err: impl Display) -> Self {
                $name(err.to_string())
            }
        }

        impl ResponseError for $name {
            fn as_response(&self) -> Response {
                Response::builder()
                    .status(StatusCode::$code)
                    .body(Body::from_string(self.0.clone()))
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        )*
    };
}

define_status_error!(
    /// An error type with a status code of [`StatusCode::BAD_REQUEST`].
    (ErrorBadRequest, BAD_REQUEST);
    /// An error type with a status code of [`StatusCode::UNAUTHORIZED`].
    (ErrorUnauthorized, UNAUTHORIZED);
    /// An error type with a status code of [`StatusCode::PAYMENT_REQUIRED`].
    (ErrorPaymentRequired, PAYMENT_REQUIRED);
    /// An error type with a status code of [`StatusCode::FORBIDDEN`].
    (ErrorForbidden, FORBIDDEN);
    /// An error type with a status code of [`StatusCode::NOT_FOUND`].
    (ErrorNotFound, NOT_FOUND);
    /// An error type with a status code of [`StatusCode::METHOD_NOT_ALLOWED`].
    (ErrorMethodNotAllowed, METHOD_NOT_ALLOWED);
    /// An error type with a status code of [`StatusCode::NOT_ACCEPTABLE`].
    (ErrorNotAcceptable, NOT_ACCEPTABLE);
    /// An error type with a status code of [`StatusCode::PROXY_AUTHENTICATION_REQUIRED`].
    (ErrorProxyAuthenticationRequired, PROXY_AUTHENTICATION_REQUIRED);
    /// An error type with a status code of [`StatusCode::REQUEST_TIMEOUT`].
    (ErrorRequestTimeout, REQUEST_TIMEOUT);
    /// An error type with a status code of [`StatusCode::CONFLICT`].
    (ErrorConflict, CONFLICT);
    /// An error type with a status code of [`StatusCode::GONE`].
    (ErrorGone, GONE);
    /// An error type with a status code of [`StatusCode::LENGTH_REQUIRED`].
    (ErrorLengthRequired, LENGTH_REQUIRED);
    /// An error type with a status code of [`StatusCode::PAYLOAD_TOO_LARGE`].
    (ErrorPayloadTooLarge, PAYLOAD_TOO_LARGE);
    /// An error type with a status code of [`StatusCode::URI_TOO_LONG`].
    (ErrorUriTooLong, URI_TOO_LONG);
    /// An error type with a status code of [`StatusCode::UNSUPPORTED_MEDIA_TYPE`].
    (ErrorUnsupportedMediaType, UNSUPPORTED_MEDIA_TYPE);
    /// An error type with a status code of [`StatusCode::RANGE_NOT_SATISFIABLE`].
    (ErrorRangeNotSatisfiable, RANGE_NOT_SATISFIABLE);
    /// An error type with a status code of [`StatusCode::IM_A_TEAPOT`].
    (ErrorImATeapot, IM_A_TEAPOT);
    /// An error type with a status code of [`StatusCode::MISDIRECTED_REQUEST`].
    (ErrorMisdirectedRequest, MISDIRECTED_REQUEST);
    /// An error type with a status code of [`StatusCode::UNPROCESSABLE_ENTITY`].
    (ErrorUnprocessableEntity, UNPROCESSABLE_ENTITY);
    /// An error type with a status code of [`StatusCode::LOCKED`].
    (ErrorLocked, LOCKED);
    /// An error type with a status code of [`StatusCode::FAILED_DEPENDENCY`].
    (ErrorFailedDependency, FAILED_DEPENDENCY);
    /// An error type with a status code of [`StatusCode::UPGRADE_REQUIRED`].
    (ErrorUpgradeRequired, UPGRADE_REQUIRED);
    /// An error type with a status code of [`StatusCode::PRECONDITION_FAILED`].
    (ErrorPreconditionFailed, PRECONDITION_FAILED);
    /// An error type with a status code of [`StatusCode::PRECONDITION_REQUIRED`].
    (ErrorPreconditionRequired, PRECONDITION_REQUIRED);
    /// An error type with a status code of [`StatusCode::TOO_MANY_REQUESTS`].
    (ErrorTooManyRequests, TOO_MANY_REQUESTS);
    /// An error type with a status code of [`StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE`].
    (ErrorRequestHeaderFieldsTooLarge, REQUEST_HEADER_FIELDS_TOO_LARGE);
    /// An error type with a status code of [`StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS`].
    (ErrorUnavailableForLegalReasons, UNAVAILABLE_FOR_LEGAL_REASONS);
    /// An error type with a status code of [`StatusCode::EXPECTATION_FAILED`].
    (ErrorExpectationFailed, EXPECTATION_FAILED);
    /// An error type with a status code of [`StatusCode::INTERNAL_SERVER_ERROR`].
    (ErrorInternalServerError, INTERNAL_SERVER_ERROR);
    /// An error type with a status code of [`StatusCode::NOT_IMPLEMENTED`].
    (ErrorNotImplemented, NOT_IMPLEMENTED);
    /// An error type with a status code of [`StatusCode::BAD_GATEWAY`].
    (ErrorBadGateway, BAD_GATEWAY);
    /// An error type with a status code of [`StatusCode::SERVICE_UNAVAILABLE`].
    (ErrorServiceUnavailable, SERVICE_UNAVAILABLE);
    /// An error type with a status code of [`StatusCode::GATEWAY_TIMEOUT`].
    (ErrorGatewayTimeout, GATEWAY_TIMEOUT);
    /// An error type with a status code of [`StatusCode::HTTP_VERSION_NOT_SUPPORTED`].
    (ErrorHttpVersionNotSupported, HTTP_VERSION_NOT_SUPPORTED);
    /// An error type with a status code of [`StatusCode::VARIANT_ALSO_NEGOTIATES`].
    (ErrorVariantAlsoNegotiates, VARIANT_ALSO_NEGOTIATES);
    /// An error type with a status code of [`StatusCode::INSUFFICIENT_STORAGE`].
    (ErrorInsufficientStorage, INSUFFICIENT_STORAGE);
    /// An error type with a status code of [`StatusCode::LOOP_DETECTED`].
    (ErrorLoopDetected, LOOP_DETECTED);
    /// An error type with a status code of [`StatusCode::NOT_EXTENDED`].
    (ErrorNotExtended, NOT_EXTENDED);
    /// An error type with a status code of [`StatusCode::NETWORK_AUTHENTICATION_REQUIRED`].
    (ErrorNetworkAuthenticationRequired, NETWORK_AUTHENTICATION_REQUIRED);
);

/// A specialized Result type for Poem.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;
