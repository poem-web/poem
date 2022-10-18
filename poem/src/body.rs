use std::{
    fmt::{Debug, Display, Formatter},
    io::{Error as IoError, ErrorKind},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{Stream, TryStreamExt};
use hyper::body::HttpBody;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    error::{ParseJsonError, ReadBodyError},
    Result,
};

/// A body object for requests and responses.
#[derive(Default)]
pub struct Body(pub(crate) hyper::Body);

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Body").finish()
    }
}

impl From<hyper::Body> for Body {
    fn from(body: hyper::Body) -> Self {
        Body(body)
    }
}

impl From<Body> for hyper::Body {
    fn from(body: Body) -> Self {
        body.0
    }
}

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

impl From<()> for Body {
    #[inline]
    fn from(_: ()) -> Self {
        Body::empty()
    }
}

impl Body {
    /// Create a body object from [`Bytes`].
    #[inline]
    pub fn from_bytes(data: Bytes) -> Self {
        data.into()
    }

    /// Create a body object from [`String`].
    #[inline]
    pub fn from_string(data: String) -> Self {
        data.into()
    }

    /// Create a body object from [`Vec<u8>`].
    #[inline]
    pub fn from_vec(data: Vec<u8>) -> Self {
        data.into()
    }

    /// Create a body object from reader.
    #[inline]
    pub fn from_async_read(reader: impl AsyncRead + Send + 'static) -> Self {
        Self(hyper::Body::wrap_stream(tokio_util::io::ReaderStream::new(
            reader,
        )))
    }

    /// Create a body object from bytes stream.
    pub fn from_bytes_stream<S, O, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<O, E>> + Send + 'static,
        O: Into<Bytes> + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        Self(hyper::Body::wrap_stream(stream))
    }

    /// Create a body object from JSON.
    pub fn from_json(body: impl Serialize) -> serde_json::Result<Self> {
        Ok(serde_json::to_vec(&body)?.into())
    }

    /// Create an empty body.
    #[inline]
    pub fn empty() -> Self {
        Self(hyper::Body::empty())
    }

    /// Returns `true` if this body is empty.
    pub fn is_empty(&self) -> bool {
        let size_hint = hyper::body::HttpBody::size_hint(&self.0);
        size_hint.lower() == 0 && size_hint.upper() == Some(0)
    }

    /// Consumes this body object to return a [`Bytes`] that contains all data.
    pub async fn into_bytes(self) -> Result<Bytes, ReadBodyError> {
        hyper::body::to_bytes(self.0)
            .await
            .map_err(|err| ReadBodyError::Io(IoError::new(ErrorKind::Other, err)))
    }

    /// Consumes this body object to return a [`Vec<u8>`] that contains all
    /// data.
    pub async fn into_vec(self) -> Result<Vec<u8>, ReadBodyError> {
        Ok(hyper::body::to_bytes(self.0)
            .await
            .map_err(|err| ReadBodyError::Io(IoError::new(ErrorKind::Other, err)))?
            .to_vec())
    }

    /// Consumes this body object to return a [`Bytes`] that contains all
    /// data, returns `Err(ReadBodyError::PayloadTooLarge)` if the length of the
    /// payload exceeds `limit`.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{error::ReadBodyError, handler, http::StatusCode, Body, Endpoint, Request, Result};
    ///
    /// #[handler]
    /// async fn index(data: Body) -> Result<()> {
    ///     Ok(data.into_bytes_limit(5).await.map(|_| ())?)
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let req = Request::builder().body("12345");
    /// assert_eq!(index.get_response(req).await.status(), StatusCode::OK);
    ///
    /// let req = Request::builder().body("123456");
    /// assert_eq!(
    ///     index.get_response(req).await.status(),
    ///     StatusCode::PAYLOAD_TOO_LARGE
    /// );
    /// # });
    /// ```
    pub async fn into_bytes_limit(self, limit: usize) -> Result<Bytes, ReadBodyError> {
        let mut reader = self.into_async_read();
        let mut buf = [0; 4096];
        let mut data = BytesMut::new();

        loop {
            let sz = reader.read(&mut buf).await?;
            if sz == 0 {
                break;
            }
            if data.len() + sz > limit {
                return Err(ReadBodyError::PayloadTooLarge);
            }
            data.extend_from_slice(&buf[..sz]);
        }

        Ok(data.freeze())
    }

    /// Consumes this body object to return a [`String`] that contains all data.
    pub async fn into_string(self) -> Result<String, ReadBodyError> {
        Ok(String::from_utf8(self.into_bytes().await?.to_vec())?)
    }

    /// Consumes this body object and parse it as `T`.
    ///
    /// # Errors
    ///
    /// - [`ReadBodyError`]
    /// - [`ParseJsonError`]
    pub async fn into_json<T: DeserializeOwned>(self) -> Result<T> {
        Ok(serde_json::from_slice(&self.into_vec().await?).map_err(ParseJsonError::Parse)?)
    }

    /// Consumes this body object and parse it as `T`.
    ///
    /// # Errors
    ///
    /// - [`ReadBodyError`]
    /// - [`ParseXmlError`](crate::error::ParseXmlError)
    #[cfg(feature = "xml")]
    pub async fn into_xml<T: DeserializeOwned>(self) -> Result<T> {
        Ok(quick_xml::de::from_reader(self.into_vec().await?.as_ref())
            .map_err(crate::error::ParseXmlError::Parse)?)
    }

    /// Consumes this body object to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Unpin + Send + 'static {
        tokio_util::io::StreamReader::new(BodyStream::new(self.0))
    }

    /// Consumes this body object to return a bytes stream.
    pub fn into_bytes_stream(self) -> impl Stream<Item = Result<Bytes, IoError>> + Send + 'static {
        TryStreamExt::map_err(self.0, |err| IoError::new(ErrorKind::Other, err))
    }
}

pin_project_lite::pin_project! {
    pub(crate) struct BodyStream<T> {
        #[pin] inner: T,
    }
}

impl<T> BodyStream<T> {
    #[inline]
    pub(crate) fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> Stream for BodyStream<T>
where
    T: HttpBody,
    T::Error: Display,
{
    type Item = Result<T::Data, std::io::Error>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_data(cx)
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create() {
        let body = Body::from(hyper::Body::from("abc"));
        assert_eq!(body.into_string().await.unwrap(), "abc");

        let body = Body::from(b"abc".as_ref());
        assert_eq!(body.into_vec().await.unwrap(), b"abc");

        let body = Body::from("abc");
        assert_eq!(body.into_string().await.unwrap(), "abc");

        let body = Body::from("abc".to_string());
        assert_eq!(body.into_string().await.unwrap(), "abc");

        let body = Body::from_string("abc".to_string());
        assert_eq!(body.into_string().await.unwrap(), "abc");

        let body = Body::from(vec![1, 2, 3]);
        assert_eq!(body.into_vec().await.unwrap(), &[1, 2, 3]);

        let body = Body::from_vec(vec![1, 2, 3]);
        assert_eq!(body.into_vec().await.unwrap(), &[1, 2, 3]);

        let body = Body::from_bytes(Bytes::from_static(b"abc"));
        assert_eq!(body.into_vec().await.unwrap(), b"abc");

        let body = Body::empty();
        assert_eq!(body.into_vec().await.unwrap(), b"");

        let body = Body::from_async_read(tokio_util::io::StreamReader::new(
            futures_util::stream::iter(
                vec![
                    Bytes::from_static(b"abc"),
                    Bytes::from_static(b"def"),
                    Bytes::from_static(b"ghi"),
                ]
                .into_iter()
                .map(Ok::<_, std::io::Error>),
            ),
        ));
        assert_eq!(body.into_string().await.unwrap(), "abcdefghi");

        let body = Body::from_json("abc").unwrap();
        assert_eq!(body.into_json::<String>().await.unwrap(), "abc");
    }
}
