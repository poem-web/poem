use std::fmt::{self, Debug, Display, Formatter};

use crate::{Body, HeaderName, Response, StatusCode};
use std::convert::Infallible;

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
    #[inline]
    pub fn new(status: StatusCode, error: impl Into<anyhow::Error>) -> Self {
        Self {
            status,
            error: error.into(),
        }
    }

    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Display + Debug + Send + Sync + 'static,
    {
        self.error.downcast_ref::<T>()
    }

    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: Display + Debug + Send + Sync + 'static,
    {
        self.error.is::<T>()
    }

    #[inline]
    pub fn is_not_found(&self) -> bool {
        self.is::<ErrorNotFound>()
    }

    pub(crate) fn as_response(&self) -> Response {
        Response::builder()
            .status(self.status)
            .header(HeaderName::CONTENT_TYPE, "text/plain")
            .body(Body::from_string(self.error.to_string()))
            .unwrap()
    }

    define_error!(
        (bad_request, BAD_REQUEST);
        (unauthorized, UNAUTHORIZED);
        (payment_required, PAYMENT_REQUIRED);
        (forbidden, FORBIDDEN);
        (not_found, NOT_FOUND);
        (method_not_allowed, METHOD_NOT_ALLOWED);
        (not_acceptable, NOT_ACCEPTABLE);
        (proxy_authentication_required, PROXY_AUTHENTICATION_REQUIRED);
        (request_timeout, REQUEST_TIMEOUT);
        (conflict, CONFLICT);
        (gone, GONE);
        (length_required, LENGTH_REQUIRED);
        (payload_too_large, PAYLOAD_TOO_LARGE);
        (uri_too_long, URI_TOO_LONG);
        (unsupported_media_type, UNSUPPORTED_MEDIA_TYPE);
        (range_not_satisfiable, RANGE_NOT_SATISFIABLE);
        (im_a_teapot, IM_A_TEAPOT);
        (misdirected_request, MISDIRECTED_REQUEST);
        (unprocessable_entity, UNPROCESSABLE_ENTITY);
        (locked, LOCKED);
        (failed_dependency, FAILED_DEPENDENCY);
        (upgrade_required, UPGRADE_REQUIRED);
        (precondition_failed, PRECONDITION_FAILED);
        (precondition_required, PRECONDITION_REQUIRED);
        (too_many_requests, TOO_MANY_REQUESTS);
        (request_header_fields_too_large, REQUEST_HEADER_FIELDS_TOO_LARGE);
        (unavailable_for_legal_reasons, UNAVAILABLE_FOR_LEGAL_REASONS);
        (expectation_failed, EXPECTATION_FAILED);
        (internal_server_error, INTERNAL_SERVER_ERROR);
        (not_implemented, NOT_IMPLEMENTED);
        (bad_gateway, BAD_GATEWAY);
        (service_unavailable, SERVICE_UNAVAILABLE);
        (gateway_timeout, GATEWAY_TIMEOUT);
        (http_version_not_supported, HTTP_VERSION_NOT_SUPPORTED);
        (variant_also_negotiates, VARIANT_ALSO_NEGOTIATES);
        (insufficient_storage, INSUFFICIENT_STORAGE);
        (loop_detected, LOOP_DETECTED);
        (not_extended, NOT_EXTENDED);
        (network_authentication_required, NETWORK_AUTHENTICATION_REQUIRED);
    );
}

macro_rules! define_simple_errors {
    ($($(#[$docs:meta])* ($name:ident, $err_msg:literal);)*) => {
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
        )*
    };
}

define_simple_errors!(
    /// ErrorNotFound
    (ErrorNotFound, "not found");

    /// ErrorInvalidMethod
    (ErrorInvalidMethod, "invalid method");

    /// ErrorInvalidHeaderName
    (ErrorInvalidHeaderName, "invalid header name");

    /// ErrorInvalidHeaderValue
    (ErrorInvalidHeaderValue, "invalid header value");

    /// ErrorInvalidMime
    (ErrorInvalidMime, "invalid mime");

    /// ErrorInvalidUri
    (ErrorInvalidUri, "invalid uri");

    /// ErrorInvalidStatusCode
    (ErrorInvalidStatusCode, "invalid status code");

    /// ErrorMissingRouteParams
    (ErrorMissingRouteParams, "missing route params");

    /// ErrorInvalidPathParams
    (ErrorInvalidPathParams, "invalid path params");
);

pub type Result<T, E = Error> = ::std::result::Result<T, E>;
