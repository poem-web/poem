//! Commonly used payload types.

mod binary;
mod json;
mod plain_text;

pub use binary::Binary;
pub use json::Json;
pub use plain_text::PlainText;
use poem::{Request, RequestBody};

use crate::{
    registry::{MetaSchemaRef, Registry},
    ParseRequestError,
};

/// Represents a payload type.
#[poem::async_trait]
pub trait Payload: Sized {
    /// The content type of this payload.
    const CONTENT_TYPE: &'static str;

    /// Gets schema reference of this payload.
    fn schema_ref() -> MetaSchemaRef;

    /// Register the schema contained in this payload to the registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}

    /// Parse the payload object from the HTTP request.
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError>;
}

#[poem::async_trait]
impl<T: Payload> Payload for poem::Result<T> {
    const CONTENT_TYPE: &'static str = T::CONTENT_TYPE;

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        match T::from_request(request, body).await {
            Ok(payload) => Ok(Ok(payload)),
            Err(err) => Ok(Err(err.into())),
        }
    }
}
