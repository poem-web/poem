use poem::http::StatusCode;
use thiserror::Error;

use crate::poem::error::{BadRequest, MethodNotAllowed};

/// This type represents errors that occur when parsing the HTTP request.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum ParseRequestError {
    /// Failed to parse a parameter.
    #[error("failed to parse param `{name}`: {reason}")]
    ParseParam {
        /// The name of the parameter.
        name: &'static str,

        /// The reason for the error.
        reason: String,
    },

    /// Failed to parse a request body.
    #[error("failed to parse request body: {reason}")]
    ParseRequestBody {
        /// The reason for the error.
        reason: String,
    },

    /// The `Content-Type` requested by the client is not supported.
    #[error("the content type `{content_type}` is not supported.")]
    ContentTypeNotSupported {
        /// The `Content-Type` header requested by the client.
        content_type: String,
    },

    /// The client request does not include the `Content-Type` header.
    #[error("expect a `Content-Type` header.")]
    ExpectContentType,

    /// Poem extractor error.
    #[error("poem extract error: {0}")]
    Extractor(String),

    /// Authorization error.
    #[error("authorization error")]
    Authorization,
}

impl From<ParseRequestError> for poem::Error {
    fn from(err: ParseRequestError) -> Self {
        match &err {
            ParseRequestError::ParseParam { .. } => BadRequest(err),
            ParseRequestError::ParseRequestBody { .. } => BadRequest(err),
            ParseRequestError::ContentTypeNotSupported { .. } => MethodNotAllowed(err),
            ParseRequestError::ExpectContentType => MethodNotAllowed(err),
            ParseRequestError::Extractor(_) => BadRequest(err),
            ParseRequestError::Authorization => poem::Error::new(StatusCode::UNAUTHORIZED),
        }
    }
}
