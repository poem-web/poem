//! Some common error types.

use poem::{error::ResponseError, http::StatusCode};
use thiserror::Error;

/// Parameter error.
#[derive(Debug, Error)]
#[error("failed to parse parameter `{name}`: {reason}")]
pub struct ParseParamError {
    /// The name of the parameter.
    pub name: &'static str,

    /// The reason for the error.
    pub reason: String,
}

impl ResponseError for ParseParamError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Parse JSON error.
#[derive(Debug, Error)]
#[error("parse JSON error: {reason}")]
pub struct ParseJsonError {
    /// The reason for the error.
    pub reason: String,
}

impl ResponseError for ParseJsonError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Parse multipart error.
#[derive(Debug, Error)]
#[error("parse multipart error: {reason}")]
pub struct ParseMultipartError {
    /// The reason for the error.
    pub reason: String,
}

impl ResponseError for ParseMultipartError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Content type error.
#[derive(Debug, Error)]
pub enum ContentTypeError {
    /// Not supported.
    #[error("the `Content-Type` requested by the client is not supported: {content_type}")]
    NotSupported {
        /// The `Content-Type` header requested by the client.
        content_type: String,
    },

    /// Expect content type header.
    #[error("the client request does not include the `Content-Type` header")]
    ExpectContentType,
}

impl ResponseError for ContentTypeError {
    fn status(&self) -> StatusCode {
        StatusCode::METHOD_NOT_ALLOWED
    }
}

/// Authorization error.
#[derive(Debug, Error)]
#[error("authorization error")]
pub struct AuthorizationError;

impl ResponseError for AuthorizationError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}
