use poem::http::StatusCode;
use thiserror::Error;

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
            ParseRequestError::ParseParam { .. } => poem::Error::bad_request(err),
            ParseRequestError::ParseRequestBody { .. } => poem::Error::bad_request(err),
            ParseRequestError::ContentTypeNotSupported { .. } => {
                poem::Error::method_not_allowed(err)
            }
            ParseRequestError::ExpectContentType => poem::Error::method_not_allowed(err),
            ParseRequestError::Extractor(_) => poem::Error::bad_request(err),
            ParseRequestError::Authorization => poem::Error::new(StatusCode::UNAUTHORIZED),
        }
    }
}
