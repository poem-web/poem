use std::io::Result as IoResult;

use futures_util::Stream;

/// Represents a type that can convert into tls config stream.
#[cfg(any(feature = "rustls", feature = "native-tls"))]
pub trait IntoTlsConfigStream<C>: Send + 'static {
    /// Represents a tls config stream.
    type Stream: Stream<Item = C> + Send + 'static;

    /// Consume itself and return tls config stream.
    fn into_stream(self) -> IoResult<Self::Stream>;
}
