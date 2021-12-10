//! Commonly used payload types.

mod attachment;
mod binary;
mod json;
mod plain_text;

use std::io::{Cursor, ErrorKind};

use poem::{error::ReadBodyError, Body, IntoResponse, Request, RequestBody, Result};
use tokio::io::AsyncReadExt;

pub use self::{
    attachment::Attachment,
    binary::{Binary, BinaryStream},
    json::Json,
    plain_text::PlainText,
};
use crate::{
    registry::{MetaSchemaRef, Registry},
    ParseRequestError,
};

/// Represents a payload type.
pub trait Payload: Send {
    /// The content type of this payload.
    const CONTENT_TYPE: &'static str;

    /// If it is `true`, it means that this payload is required.
    const IS_REQUIRED: bool = true;

    /// Gets schema reference of this payload.
    fn schema_ref() -> MetaSchemaRef;

    /// Register the schema contained in this payload to the registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}
}

/// Represents a payload that can parse from HTTP request.
#[poem::async_trait]
pub trait ParsePayload: Sized {
    /// Parse the payload object from the HTTP request.
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError>;
}

impl<T: Payload> Payload for Option<T> {
    const CONTENT_TYPE: &'static str = T::CONTENT_TYPE;

    const IS_REQUIRED: bool = false;

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

#[poem::async_trait]
impl<T: ParsePayload> ParsePayload for Option<T> {
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        let taked_body = body
            .take()
            .map_err(|err| ParseRequestError::ParseRequestBody(err.into_response()))?;
        let mut body_reader = taked_body.into_async_read();

        match body_reader.read_u8().await {
            Ok(ch) => {
                *body =
                    RequestBody::new(Body::from_async_read(Cursor::new([ch]).chain(body_reader)));
                T::from_request(request, body).await.map(Some)
            }
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(ParseRequestError::ParseRequestBody(
                        ReadBodyError::Io(err).into_response(),
                    ))
                }
            }
        }
    }
}
