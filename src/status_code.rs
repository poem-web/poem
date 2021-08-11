use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};

use crate::error::{Error, ErrorInvalidStatusCode};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StatusCode(pub(crate) http::StatusCode);

impl TryFrom<u16> for StatusCode {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(Self(http::StatusCode::try_from(value).map_err(|_| {
            Error::internal_server_error(ErrorInvalidStatusCode)
        })?))
    }
}

impl Display for StatusCode {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! define_status_codes {
    ($($(#[$docs:meta])* $name:ident;)*) => {
        $(
        $(#[$docs])*
        pub const $name: StatusCode = StatusCode(http::StatusCode::$name);
        )*
    };
}

impl StatusCode {
    #[inline]
    pub fn as_u16(&self) -> u16 {
        self.0.as_u16()
    }

    #[inline]
    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    #[inline]
    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    define_status_codes!(
        /// 100 Continue
        /// [[RFC7231, Section 6.2.1](https://tools.ietf.org/html/rfc7231#section-6.2.1)]
        CONTINUE;
        /// 101 Switching Protocols
        /// [[RFC7231, Section 6.2.2](https://tools.ietf.org/html/rfc7231#section-6.2.2)]
        SWITCHING_PROTOCOLS;
        /// 102 Processing
        /// [[RFC2518](https://tools.ietf.org/html/rfc2518)]
        PROCESSING;

        /// 200 OK
        /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
        OK;
        /// 201 Created
        /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
        CREATED;
        /// 202 Accepted
        /// [[RFC7231, Section 6.3.3](https://tools.ietf.org/html/rfc7231#section-6.3.3)]
        ACCEPTED;
        /// 203 Non-Authoritative Information
        /// [[RFC7231, Section 6.3.4](https://tools.ietf.org/html/rfc7231#section-6.3.4)]
        NON_AUTHORITATIVE_INFORMATION;
        /// 204 No Content
        /// [[RFC7231, Section 6.3.5](https://tools.ietf.org/html/rfc7231#section-6.3.5)]
        NO_CONTENT;
        /// 205 Reset Content
        /// [[RFC7231, Section 6.3.6](https://tools.ietf.org/html/rfc7231#section-6.3.6)]
        RESET_CONTENT;
        /// 206 Partial Content
        /// [[RFC7233, Section 4.1](https://tools.ietf.org/html/rfc7233#section-4.1)]
        PARTIAL_CONTENT;
        /// 207 Multi-Status
        /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
        MULTI_STATUS;
        /// 208 Already Reported
        /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
        ALREADY_REPORTED;
        /// 226 IM Used
        /// [[RFC3229](https://tools.ietf.org/html/rfc3229)]
        IM_USED;
        /// 300 Multiple Choices
        /// [[RFC7231, Section 6.4.1](https://tools.ietf.org/html/rfc7231#section-6.4.1)]
        MULTIPLE_CHOICES;
        /// 301 Moved Permanently
        /// [[RFC7231, Section 6.4.2](https://tools.ietf.org/html/rfc7231#section-6.4.2)]
        MOVED_PERMANENTLY;
        /// [[RFC7231, Section 6.4.3](https://tools.ietf.org/html/rfc7231#section-6.4.3)]
        FOUND;
        /// 303 See Other
        /// [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]
        SEE_OTHER;
        /// 304 Not Modified
        /// [[RFC7232, Section 4.1](https://tools.ietf.org/html/rfc7232#section-4.1)]
        NOT_MODIFIED;
        /// 305 Use Proxy
        /// [[RFC7231, Section 6.4.5](https://tools.ietf.org/html/rfc7231#section-6.4.5)]
        USE_PROXY;
        /// 307 Temporary Redirect
        /// [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]
        TEMPORARY_REDIRECT;
        /// 308 Permanent Redirect
        /// [[RFC7238](https://tools.ietf.org/html/rfc7238)]
        PERMANENT_REDIRECT;

        /// 400 Bad Request
        /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
        BAD_REQUEST;
        /// 401 Unauthorized
        /// [[RFC7235, Section 3.1](https://tools.ietf.org/html/rfc7235#section-3.1)]
        UNAUTHORIZED;
        /// 402 Payment Required
        /// [[RFC7231, Section 6.5.2](https://tools.ietf.org/html/rfc7231#section-6.5.2)]
        PAYMENT_REQUIRED;
        /// 403 Forbidden
        /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
        FORBIDDEN;
        /// 404 Not Found
        /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
        NOT_FOUND;
        /// 405 Method Not Allowed
        /// [[RFC7231, Section 6.5.5](https://tools.ietf.org/html/rfc7231#section-6.5.5)]
        METHOD_NOT_ALLOWED;
        /// 406 Not Acceptable
        /// [[RFC7231, Section 6.5.6](https://tools.ietf.org/html/rfc7231#section-6.5.6)]
        NOT_ACCEPTABLE;
        /// 407 Proxy Authentication Required
        /// [[RFC7235, Section 3.2](https://tools.ietf.org/html/rfc7235#section-3.2)]
        PROXY_AUTHENTICATION_REQUIRED;
        /// 408 Request Timeout
        /// [[RFC7231, Section 6.5.7](https://tools.ietf.org/html/rfc7231#section-6.5.7)]
        REQUEST_TIMEOUT;
        /// 409 Conflict
        /// [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]
        CONFLICT;
        /// 410 Gone
        /// [[RFC7231, Section 6.5.9](https://tools.ietf.org/html/rfc7231#section-6.5.9)]
        GONE;
        /// 411 Length Required
        /// [[RFC7231, Section 6.5.10](https://tools.ietf.org/html/rfc7231#section-6.5.10)]
        LENGTH_REQUIRED;
        /// 412 Precondition Failed
        /// [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]
        PRECONDITION_FAILED;
        /// 413 Payload Too Large
        /// [[RFC7231, Section 6.5.11](https://tools.ietf.org/html/rfc7231#section-6.5.11)]
        PAYLOAD_TOO_LARGE;
        /// 414 URI Too Long
        /// [[RFC7231, Section 6.5.12](https://tools.ietf.org/html/rfc7231#section-6.5.12)]
        URI_TOO_LONG;
        /// 415 Unsupported Media Type
        /// [[RFC7231, Section 6.5.13](https://tools.ietf.org/html/rfc7231#section-6.5.13)]
        UNSUPPORTED_MEDIA_TYPE;
        /// 416 Range Not Satisfiable
        /// [[RFC7233, Section 4.4](https://tools.ietf.org/html/rfc7233#section-4.4)]
        RANGE_NOT_SATISFIABLE;
        /// 417 Expectation Failed
        /// [[RFC7231, Section 6.5.14](https://tools.ietf.org/html/rfc7231#section-6.5.14)]
        EXPECTATION_FAILED;
        /// 418 I'm a teapot
        /// [curiously not registered by IANA but [RFC2324](https://tools.ietf.org/html/rfc2324)]
        IM_A_TEAPOT;

        /// 421 Misdirected Request
        /// [RFC7540, Section 9.1.2](http://tools.ietf.org/html/rfc7540#section-9.1.2)
        MISDIRECTED_REQUEST;
        /// 422 Unprocessable Entity
        /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
        UNPROCESSABLE_ENTITY;
        /// 423 Locked
        /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
        LOCKED;
        /// 424 Failed Dependency
        /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
        FAILED_DEPENDENCY;
        /// 426 Upgrade Required
        /// [[RFC7231, Section 6.5.15](https://tools.ietf.org/html/rfc7231#section-6.5.15)]
        UPGRADE_REQUIRED;
        /// 428 Precondition Required
        /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
        PRECONDITION_REQUIRED;
        /// 429 Too Many Requests
        /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
        TOO_MANY_REQUESTS;
        /// 431 Request Header Fields Too Large
        /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
        REQUEST_HEADER_FIELDS_TOO_LARGE;
        /// 451 Unavailable For Legal Reasons
        /// [[RFC7725](http://tools.ietf.org/html/rfc7725)]
        UNAVAILABLE_FOR_LEGAL_REASONS;

        /// 500 Internal Server Error
        /// [[RFC7231, Section 6.6.1](https://tools.ietf.org/html/rfc7231#section-6.6.1)]
        INTERNAL_SERVER_ERROR;
        /// 501 Not Implemented
        /// [[RFC7231, Section 6.6.2](https://tools.ietf.org/html/rfc7231#section-6.6.2)]
        NOT_IMPLEMENTED;
        /// 502 Bad Gateway
        /// [[RFC7231, Section 6.6.3](https://tools.ietf.org/html/rfc7231#section-6.6.3)]
        BAD_GATEWAY;
        /// 503 Service Unavailable
        /// [[RFC7231, Section 6.6.4](https://tools.ietf.org/html/rfc7231#section-6.6.4)]
        SERVICE_UNAVAILABLE;
        /// 504 Gateway Timeout
        /// [[RFC7231, Section 6.6.5](https://tools.ietf.org/html/rfc7231#section-6.6.5)]
        GATEWAY_TIMEOUT;
        /// 505 HTTP Version Not Supported
        /// [[RFC7231, Section 6.6.6](https://tools.ietf.org/html/rfc7231#section-6.6.6)]
        HTTP_VERSION_NOT_SUPPORTED;
        /// 506 Variant Also Negotiates
        /// [[RFC2295](https://tools.ietf.org/html/rfc2295)]
        VARIANT_ALSO_NEGOTIATES;
        /// 507 Insufficient Storage
        /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
        INSUFFICIENT_STORAGE;
        /// 508 Loop Detected
        /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
        LOOP_DETECTED;
        /// 510 Not Extended
        /// [[RFC2774](https://tools.ietf.org/html/rfc2774)]
        NOT_EXTENDED;
        /// 511 Network Authentication Required
        /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
        NETWORK_AUTHENTICATION_REQUIRED;
    );
}
