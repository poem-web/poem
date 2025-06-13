//! Content type.

use serde::Serialize;

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
}
