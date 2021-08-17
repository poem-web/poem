use hyper::http::HeaderValue;
use tokio::io::BufReader;

use crate::{http::header, Body, Endpoint, Middleware, Request, Response, Result};

/// The compression algorithms.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompressionAlgo {
    /// brotli
    BR,

    /// deflate
    DEFLATE,

    /// gzip
    GZIP,
}

/// Middleware for decompress the request body.
pub struct Decompress;

impl<E: Endpoint> Middleware<E> for Decompress {
    type Output = DecompressImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        DecompressImpl { inner: ep }
    }
}

#[doc(hidden)]
pub struct DecompressImpl<E> {
    inner: E,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for DecompressImpl<E> {
    async fn call(&self, mut req: Request) -> Result<Response> {
        let encoding = req
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        match encoding {
            "br" => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliDecoder::new(BufReader::new(body)),
                ));
            }
            "deflate" => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::DeflateDecoder::new(BufReader::new(body)),
                ));
            }
            "gzip" => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(body)),
                ));
            }
            _ => {}
        }

        self.inner.call(req).await
    }
}

/// Middleware for compresses the response body with the specified algorithm.
pub struct Compress {
    algo: CompressionAlgo,
}

impl Compress {
    /// Creates Compress middleware used specified algorithm.
    pub fn new(algo: CompressionAlgo) -> Self {
        Self { algo }
    }
}

impl<E: Endpoint> Middleware<E> for Compress {
    type Output = CompressImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        CompressImpl {
            inner: ep,
            algo: self.algo,
        }
    }
}

#[doc(hidden)]
pub struct CompressImpl<E> {
    inner: E,
    algo: CompressionAlgo,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CompressImpl<E> {
    async fn call(&self, req: Request) -> Result<Response> {
        let mut resp = self.inner.call(req).await?;
        let body = resp.take_body()?.into_async_read();

        match self.algo {
            CompressionAlgo::BR => {
                resp.headers_mut()
                    .append(header::CONTENT_ENCODING, HeaderValue::from_static("br"));
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliEncoder::new(BufReader::new(body)),
                ));
            }
            CompressionAlgo::DEFLATE => {
                resp.headers_mut().append(
                    header::CONTENT_ENCODING,
                    HeaderValue::from_static("deflate"),
                );
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::DeflateEncoder::new(BufReader::new(body)),
                ));
            }
            CompressionAlgo::GZIP => {
                resp.headers_mut()
                    .append(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::GzipEncoder::new(BufReader::new(body)),
                ));
            }
        }

        Ok(resp)
    }
}
