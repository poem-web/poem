//! Commonly used payload types.

mod attachment;
mod binary;
mod event_stream;
mod json;
mod plain_text;
mod response;

use std::str::FromStr;

use mime::Mime;
use poem::{Request, RequestBody, Result};

pub use self::{
    attachment::Attachment, binary::Binary, event_stream::EventStream, json::Json,
    plain_text::PlainText, response::Response,
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

#[doc(hidden)]
pub struct ContentTypeTable {
    items: Vec<(Mime, usize)>,
}

impl ContentTypeTable {
    pub fn new(types: &[&str]) -> Self {
        let mut items = types
            .iter()
            .enumerate()
            .map(|(idx, s)| (Mime::from_str(s).unwrap(), idx))
            .collect::<Vec<_>>();

        items.sort_by_key(|(x, _)| {
            let mut n = 0;
            if x.type_() == mime::STAR {
                n += 1;
            }
            if x.subtype() == mime::STAR {
                n += 2;
            }
            n
        });

        ContentTypeTable { items }
    }

    pub fn matches(&self, content_type: &str) -> Option<usize> {
        for (mime, idx) in &self.items {
            if let Ok(x) = Mime::from_str(content_type) {
                if (x.type_() == mime.type_() || mime.type_() == mime::STAR)
                    && (x.subtype() == mime.subtype() || mime.subtype() == mime::STAR)
                {
                    return Some(*idx);
                }
            }
        }
        None
    }
}
