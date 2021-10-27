use poem::{http::StatusCode, Error};

/// This type represents errors that occur when parsing the HTTP request.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseRequestError {
    /// Failed to parse a parameter.
    ParseParam {
        /// The name of the parameter.
        name: &'static str,

        /// The reason for the error.
        reason: String,
    },

    /// Failed to parse a request body.
    ParseRequestBody {
        /// The reason for the error.
        reason: String,
    },

    /// The `Content-Type` requested by the client is not supported.
    ContentTypeNotSupported {
        /// The `Content-Type` header requested by the client.
        content_type: String,
    },

    /// The client request does not include the `Content-Type` header.
    ExpectContentType,

    /// Poem extractor error.
    Extractor(String),

    /// Authorization error.
    Authorization,
}

#[allow(clippy::inherent_to_string)]
impl ParseRequestError {
    /// Convert this error to string.
    pub fn to_string(&self) -> String {
        Into::<Error>::into(self.clone())
            .reason()
            .unwrap_or_default()
            .to_string()
    }
}

impl From<ParseRequestError> for poem::Error {
    fn from(err: ParseRequestError) -> Self {
        match err {
            ParseRequestError::ParseParam { name, reason } => Error::new(StatusCode::BAD_REQUEST)
                .with_reason(format!("failed to parse param `{}`: {}", name, reason)),
            ParseRequestError::ParseRequestBody { reason } => Error::new(StatusCode::BAD_REQUEST)
                .with_reason(format!("failed to parse request body: {}", reason)),
            ParseRequestError::ContentTypeNotSupported { content_type } => {
                Error::new(StatusCode::METHOD_NOT_ALLOWED).with_reason(format!(
                    "the content type `{}` is not supported.",
                    content_type
                ))
            }
            ParseRequestError::ExpectContentType => Error::new(StatusCode::METHOD_NOT_ALLOWED)
                .with_reason("expect a `Content-Type` header."),
            ParseRequestError::Extractor(err) => Error::new(StatusCode::BAD_REQUEST)
                .with_reason(format!("poem extract error: {}", err)),
            ParseRequestError::Authorization => {
                Error::new(StatusCode::UNAUTHORIZED).with_reason("authorization error")
            }
        }
    }
}
