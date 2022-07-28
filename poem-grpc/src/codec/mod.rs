//! Generic encoding and decoding.

#[cfg(feature = "json-codec")]
mod json_codec;
mod prost_codec;

use std::io::Result;

use bytes::BytesMut;

/// The encoder that can encode a message
pub trait Encoder: Send + 'static {
    /// The message type
    type Item: Send + 'static;

    /// Encode a message to buffer
    fn encode(&mut self, message: Self::Item, buf: &mut BytesMut) -> Result<()>;
}

/// The decoder that can decode a message
pub trait Decoder: Send + 'static {
    /// The message type
    type Item: Send + 'static;

    /// Decode a message from buffer
    fn decode(&mut self, buf: &[u8]) -> Result<Self::Item>;
}

/// Represents a type that can encode/decode a message
pub trait Codec: Default {
    /// Content types
    const CONTENT_TYPES: &'static [&'static str];

    /// The encodable message
    type Encode: Send + 'static;

    /// The decodable message
    type Decode: Send + 'static;

    /// The encoder that can encode a message
    type Encoder: Encoder<Item = Self::Encode>;

    /// The decoder that can encode a message
    type Decoder: Decoder<Item = Self::Decode>;

    /// Returns whether the specified content type is supported
    #[inline]
    fn check_content_type(&self, ct: &str) -> bool {
        Self::CONTENT_TYPES.iter().any(|value| *value == ct)
    }

    /// Create the encoder
    fn encoder(&mut self) -> Self::Encoder;

    /// Create the decoder
    fn decoder(&mut self) -> Self::Decoder;
}

#[cfg(feature = "json-codec")]
pub use json_codec::{
    JsonCodec, JsonDecoder, JsonEncoder, JsonI64ToStringCodec, JsonI64ToStringDecoder,
    JsonI64ToStringEncoder,
};
pub use prost_codec::{ProstCodec, ProstDecoder, ProstEncoder};
