//! Commonly used payload types.

mod attachment;
mod base64_payload;
mod binary;
mod event_stream;
mod form;
mod html;
mod json;
mod plain_text;
mod response;
mod xml;

use poem::{Request, RequestBody, Result};

pub use self::{
    attachment::{Attachment, AttachmentType},
    base64_payload::Base64,
    binary::Binary,
    event_stream::EventStream,
    form::Form,
    html::Html,
    json::Json,
    plain_text::PlainText,
    response::Response,
    xml::Xml,
};
use crate::registry::{MetaSchemaRef, Registry};

/// Represents a payload type.
pub trait Payload: Send {
    /// The content type of this payload.
    const CONTENT_TYPE: &'static str;

    /// Gets schema reference of this payload.
    fn schema_ref() -> MetaSchemaRef;

    /// Register the schema contained in this payload to the registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}
}

/// Represents a payload that can parse from HTTP request.
#[poem::async_trait]
pub trait ParsePayload: Sized {
    /// If it is `true`, it means that this payload is required.
    const IS_REQUIRED: bool;

    /// Parse the payload object from the HTTP request.
    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self>;
}
