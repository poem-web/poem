use std::str::FromStr;

use hyper::http::HeaderValue;
use tokio::io::BufReader;

use crate::{http::header, Body, Endpoint, Error, Middleware, Request, Response, Result};

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

/// Middleware for compression.
///
/// It decompresses the request body according to `Content-Encoding`, and
/// compresses the response body according to `Accept-Encoding`.
///
/// You can also specify the compression algorithm [`CompressionAlgo`] yourself,
/// so it will always use this algorithm to compress the response body and add
/// the `Accept-Encoding` header.
pub struct Compression {
    compress_algo: Option<CompressionAlgo>,
}

impl Compression {
    /// Specify the compression algorithm for the response body.
    pub fn algorithm(self, algo: CompressionAlgo) -> Self {
        Self {
            compress_algo: Some(algo),
        }
    }
}

impl<E: Endpoint> Middleware<E> for Compression {
    type Output = CompressionImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        CompressionImpl {
            inner: ep,
            compress_algo: self.compress_algo,
        }
    }
}

#[doc(hidden)]
pub struct CompressionImpl<E> {
    inner: E,
    compress_algo: Option<CompressionAlgo>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CompressionImpl<E> {
    async fn call(&self, mut req: Request) -> Result<Response> {
        let encoding = match req
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok())
        {
            Some(encoding) => Some(encoding.parse::<CompressionAlgo>().map_err(|_| {
                Error::bad_request(anyhow::anyhow!(
                    "unsupported compression algorithm in `Content-Encoding`: `{}`",
                    encoding
                ))
            })?),
            None => None,
        };

        let accept_encoding = match req
            .headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|value| value.to_str().ok())
        {
            Some(encoding) => Some(encoding.parse::<CompressionAlgo>().map_err(|_| {
                Error::bad_request(anyhow::anyhow!(
                    "unsupported compression algorithm in `Accept-Encoding`: `{}`",
                    encoding
                ))
            })?),
            None => None,
        };

        match encoding {
            Some(CompressionAlgo::BR) => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliDecoder::new(BufReader::new(body)),
                ));
            }
            Some(CompressionAlgo::DEFLATE) => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::DeflateDecoder::new(BufReader::new(body)),
                ));
            }
            Some(CompressionAlgo::GZIP) => {
                let body = req.take_body()?.into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(body)),
                ));
            }
            None => {}
        }

        let mut resp = self.inner.call(req).await?;

        match accept_encoding.or(self.compress_algo) {
            Some(CompressionAlgo::BR) => {
                let body = resp.take_body()?;
                resp.headers_mut()
                    .append(header::CONTENT_ENCODING, HeaderValue::from_static("br"));
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliEncoder::new(BufReader::new(
                        body.into_async_read(),
                    )),
                ));
            }
            Some(CompressionAlgo::DEFLATE) => {
                let body = resp.take_body()?;
                resp.headers_mut().append(
                    header::CONTENT_ENCODING,
                    HeaderValue::from_static("deflate"),
                );
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::DeflateEncoder::new(BufReader::new(
                        body.into_async_read(),
                    )),
                ));
            }
            Some(CompressionAlgo::GZIP) => {
                let body = resp.take_body()?;
                resp.headers_mut()
                    .append(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::GzipEncoder::new(BufReader::new(
                        body.into_async_read(),
                    )),
                ));
            }
            None => {}
        }

        Ok(resp)
    }
}
