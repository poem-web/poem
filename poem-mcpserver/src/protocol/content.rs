//! Content type.

use serde::Serialize;
use serde_json::Value;

/// A content that can be sent to the client.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Content {
    /// A text content.
    Text {
        /// The text content.
        text: String,
    },
    /// An image content.
    Image {
        /// The image data.
        data: String,
        /// The MIME type of the image.
        mime_type: String,
    },
    /// A link to a resource.
    ResourceLink {
        /// The URI of the resource.
        uri: String,
        /// The name of the resource.
        name: String,
        /// The description of the resource.
        description: String,
        /// The MIME type of the resource.
        mime_type: String,
        /// Additional annotations for the resource.
        annotations: Value,
    },
}
