use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use hyper::body::HttpBody;
use tokio::io::AsyncRead;

use crate::error::{Error, Result};

/// A body object for requests and responses.
#[derive(Default)]
pub struct Body(pub(crate) hyper::Body);

impl From<&'static [u8]> for Body {
    #[inline]
    fn from(data: &'static [u8]) -> Self {
        Self(data.into())
    }
}

impl From<&'static str> for Body {
    #[inline]
    fn from(data: &'static str) -> Self {
        Self(data.into())
    }
}

impl From<Bytes> for Body {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self(data.into())
    }
}

impl From<Vec<u8>> for Body {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self(data.into())
    }
}

impl From<String> for Body {
    #[inline]
    fn from(data: String) -> Self {
        Self(data.into())
    }
}

impl Body {
    /// Create a body objecj from [`Bytes`].
    #[inline]
    pub fn from_bytes(data: Bytes) -> Self {
        data.into()
    }

    /// Create a body objecj from [`String`].
    #[inline]
    pub fn from_string(data: String) -> Self {
        data.into()
    }

    /// Create a body object from reader.
    #[inline]
    pub fn from_async_read(reader: impl AsyncRead + Send + 'static) -> Self {
        Self(hyper::Body::wrap_stream(tokio_util::io::ReaderStream::new(
            reader,
        )))
    }

    /// Create an empty body.
    #[inline]
    pub fn empty() -> Self {
        Self(hyper::Body::empty())
    }

    /// Consumes this body object to return a [`Bytes`] that contains all data.
    pub async fn into_bytes(self) -> Result<Bytes> {
        hyper::body::to_bytes(self.0)
            .await
            .map_err(Error::internal_server_error)
    }

    /// Consumes this body object to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Send + 'static {
        tokio_util::io::StreamReader::new(BodyStream(self.0))
    }
}

struct BodyStream(hyper::Body);

impl Stream for BodyStream {
    type Item = Result<Bytes, std::io::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0)
            .poll_data(cx)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}
