use poem::{http::StatusCode, IntoResponse, Response};
use thiserror::Error;

/// This type represents errors that occur when parsing the HTTP request.
#[derive(Debug, Error)]
pub enum ParseRequestError {
    /// Failed to parse a parameter.
    #[error("Failed to parse parameter `{name}`: {reason}")]
    ParseParam {
        /// The name of the parameter.
        name: &'static str,

        /// The reason for the error.
        reason: String,
    },

    /// Failed to parse a request body.
    #[error("Failed to parse a request body")]
    ParseRequestBody(Response),

    /// The `Content-Type` requested by the client is not supported.
    #[error("The `Content-Type` requested by the client is not supported: {content_type}")]
    ContentTypeNotSupported {
        /// The `Content-Type` header requested by the client.
        content_type: String,
    },

    /// The client request does not include the `Content-Type` header.
    #[error("The client request does not include the `Content-Type` header")]
    ExpectContentType,

    /// Poem extractor error.
    #[error("Poem extractor error")]
    Extractor(Response),

    /// Authorization error.
    #[error("Authorization error")]
    Authorization,
}

impl IntoResponse for ParseRequestError {
    fn into_response(self) -> Response {
        match self {
            ParseRequestError::ParseParam { .. } => self
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
            ParseRequestError::ContentTypeNotSupported { .. }
            | ParseRequestError::ExpectContentType => self
                .to_string()
                .with_status(StatusCode::METHOD_NOT_ALLOWED)
                .into_response(),
            ParseRequestError::ParseRequestBody(resp) | ParseRequestError::Extractor(resp) => resp,
            ParseRequestError::Authorization => self
                .to_string()
                .with_status(StatusCode::UNAUTHORIZED)
                .into_response(),
        }
    }
}
