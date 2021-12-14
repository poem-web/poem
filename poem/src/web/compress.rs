use std::{
    fmt::{self, Display, Formatter},
    pin::Pin,
    str::FromStr,
};

use tokio::io::{AsyncRead, BufReader};

use crate::{
    http::{header, HeaderValue},
    Body, IntoResponse, Response,
};

/// The compression algorithms.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompressionAlgo {
    /// brotli
    BR,

    /// deflate
    DEFLATE,

    /// gzip
    GZIP,
}

impl FromStr for CompressionAlgo {
    type Err = ();

    fn from_str(s: &str) -> std::prelude::rust_2015::Result<Self, Self::Err> {
        Ok(match s {
            "br" => CompressionAlgo::BR,
            "deflate" => CompressionAlgo::DEFLATE,
            "gzip" => CompressionAlgo::GZIP,
            _ => return Err(()),
        })
    }
}

impl CompressionAlgo {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            CompressionAlgo::BR => "br",
            CompressionAlgo::DEFLATE => "deflate",
            CompressionAlgo::GZIP => "gzip",
        }
    }

    pub(crate) fn compress<'a>(
        &self,
        reader: impl AsyncRead + Send + Unpin + 'a,
    ) -> Pin<Box<dyn AsyncRead + Send + 'a>> {
        match self {
            CompressionAlgo::BR => Box::pin(async_compression::tokio::bufread::BrotliEncoder::new(
                BufReader::new(reader),
            )),
            CompressionAlgo::DEFLATE => Box::pin(
                async_compression::tokio::bufread::DeflateEncoder::new(BufReader::new(reader)),
            ),
            CompressionAlgo::GZIP => Box::pin(async_compression::tokio::bufread::GzipEncoder::new(
                BufReader::new(reader),
            )),
        }
    }

    pub(crate) fn decompress<'a>(
        &self,
        reader: impl AsyncRead + Send + Unpin + 'a,
    ) -> Pin<Box<dyn AsyncRead + Send + 'a>> {
        match self {
            CompressionAlgo::BR => Box::pin(async_compression::tokio::bufread::BrotliDecoder::new(
                BufReader::new(reader),
            )),
            CompressionAlgo::DEFLATE => Box::pin(
                async_compression::tokio::bufread::DeflateDecoder::new(BufReader::new(reader)),
            ),
            CompressionAlgo::GZIP => Box::pin(async_compression::tokio::bufread::GzipDecoder::new(
                BufReader::new(reader),
            )),
        }
    }
}

impl Display for CompressionAlgo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Compress the response body with the specified algorithm and set the
/// `Content-Encoding` header.
///
/// # Example
///
/// ```
/// use poem::{
///     handler,
///     web::{Compress, CompressionAlgo},
/// };
///
/// #[handler]
/// fn index() -> Compress<String> {
///     Compress::new("abcdef".to_string(), CompressionAlgo::GZIP)
/// }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
pub struct Compress<T> {
    inner: T,
    algo: CompressionAlgo,
}

impl<T> Compress<T> {
    /// /// Create a compressed response using the specified algorithm.
    pub fn new(inner: T, algo: CompressionAlgo) -> Self {
        Self { inner, algo }
    }
}

impl<T: IntoResponse> IntoResponse for Compress<T> {
    fn into_response(self) -> Response {
        let mut resp = self.inner.into_response();
        let body = resp.take_body();

        resp.headers_mut().append(
            header::CONTENT_ENCODING,
            HeaderValue::from_static(self.algo.as_str()),
        );

        resp.set_body(Body::from_async_read(
            self.algo.compress(body.into_async_read()),
        ));
        resp
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::{handler, Endpoint, EndpointExt, Request};

    async fn decompress_data(algo: CompressionAlgo, data: &[u8]) -> String {
        let mut output = Vec::new();

        let mut dec = algo.decompress(data);
        dec.read_to_end(&mut output).await.unwrap();
        String::from_utf8(output).unwrap()
    }

    async fn test_algo(algo: CompressionAlgo) {
        const DATA: &str = "abcdefghijklmnopqrstuvwxyz1234567890";

        #[handler(internal)]
        async fn index() -> &'static str {
            DATA
        }

        let mut resp = index
            .and_then(move |resp| async move { Ok(Compress::new(resp, algo)) })
            .call(Request::default())
            .await
            .unwrap()
            .into_response();
        assert_eq!(
            resp.headers().get(header::CONTENT_ENCODING),
            Some(&HeaderValue::from_static(algo.as_str()))
        );
        assert_eq!(
            decompress_data(algo, &resp.take_body().into_bytes().await.unwrap()).await,
            DATA
        );
    }

    #[tokio::test]
    async fn test_compress() {
        test_algo(CompressionAlgo::BR).await;
        test_algo(CompressionAlgo::DEFLATE).await;
        test_algo(CompressionAlgo::GZIP).await;
    }
}
