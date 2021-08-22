use std::str::FromStr;

use hyper::http::HeaderValue;
use tokio::io::BufReader;

use crate::{
    http::header, Body, Endpoint, Error, IntoResponse, Middleware, Request, Response, Result,
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

/// Middleware for compression.
///
/// It decompresses the request body according to `Content-Encoding`, and
/// compresses the response body according to `Accept-Encoding`.
///
/// You can also specify the compression algorithm [`CompressionAlgo`] yourself,
/// so it will always use this algorithm to compress the response body and add
/// the `Accept-Encoding` header.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
#[derive(Default)]
pub struct Compression {
    compress_algo: Option<CompressionAlgo>,
}

impl Compression {
    /// Create new `Compression` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self {
            compress_algo: None,
        }
    }

    /// Specify the compression algorithm for the response body.
    #[must_use]
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

impl<E: Endpoint> CompressionImpl<E> {
    async fn do_call(&self, mut req: Request) -> Result<Response> {
        let encoding = match req
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok())
        {
            Some(encoding) => Some(encoding.parse::<CompressionAlgo>().map_err(|_| {
                Error::bad_request(format!(
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
                Error::bad_request(format!(
                    "unsupported compression algorithm in `Accept-Encoding`: `{}`",
                    encoding
                ))
            })?),
            None => None,
        };

        match encoding {
            Some(CompressionAlgo::BR) => {
                let body = req.take_body().into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliDecoder::new(BufReader::new(body)),
                ));
            }
            Some(CompressionAlgo::DEFLATE) => {
                let body = req.take_body().into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::DeflateDecoder::new(BufReader::new(body)),
                ));
            }
            Some(CompressionAlgo::GZIP) => {
                let body = req.take_body().into_async_read();
                req.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(body)),
                ));
            }
            None => {}
        }

        let mut resp = self.inner.call(req).await;
        if !resp.status().is_success() {
            return Ok(resp);
        }

        match accept_encoding.or(self.compress_algo) {
            Some(CompressionAlgo::BR) => {
                let body = resp.take_body();
                resp.headers_mut()
                    .append(header::CONTENT_ENCODING, HeaderValue::from_static("br"));
                resp.set_body(Body::from_async_read(
                    async_compression::tokio::bufread::BrotliEncoder::new(BufReader::new(
                        body.into_async_read(),
                    )),
                ));
            }
            Some(CompressionAlgo::DEFLATE) => {
                let body = resp.take_body();
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
                let body = resp.take_body();
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

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CompressionImpl<E> {
    async fn call(&self, req: Request) -> Response {
        self.do_call(req).await.into_response()
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::{handler, EndpointExt};

    async fn compress_data(algo: CompressionAlgo, data: &str) -> Vec<u8> {
        let mut output = Vec::new();

        match algo {
            CompressionAlgo::BR => {
                let mut enc = async_compression::tokio::write::BrotliEncoder::new(&mut output);
                enc.write_all(data.as_bytes()).await.unwrap();
                enc.flush().await.unwrap();
                enc.shutdown().await.unwrap();
            }
            CompressionAlgo::DEFLATE => {
                let mut enc = async_compression::tokio::write::DeflateEncoder::new(&mut output);
                enc.write_all(data.as_bytes()).await.unwrap();
                enc.flush().await.unwrap();
                enc.shutdown().await.unwrap();
            }
            CompressionAlgo::GZIP => {
                let mut enc = async_compression::tokio::write::GzipEncoder::new(&mut output);
                enc.write_all(data.as_bytes()).await.unwrap();
                enc.flush().await.unwrap();
                enc.shutdown().await.unwrap();
            }
        }

        output
    }

    async fn decompress_data(algo: CompressionAlgo, data: &[u8]) -> String {
        let mut output = Vec::new();

        match algo {
            CompressionAlgo::BR => {
                let mut dec =
                    async_compression::tokio::bufread::BrotliDecoder::new(BufReader::new(data));
                dec.read_to_end(&mut output).await.unwrap();
            }
            CompressionAlgo::DEFLATE => {
                let mut dec =
                    async_compression::tokio::bufread::DeflateDecoder::new(BufReader::new(data));
                dec.read_to_end(&mut output).await.unwrap();
            }
            CompressionAlgo::GZIP => {
                let mut dec =
                    async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(data));
                dec.read_to_end(&mut output).await.unwrap();
            }
        }

        String::from_utf8(output).unwrap()
    }

    #[tokio::test]
    async fn test_compression() {
        const DATA: &str = "abcdefghijklmnopqrstuvwxyz1234567890";

        #[handler(internal)]
        async fn index(data: String) -> String {
            assert_eq!(data, DATA);
            data
        }

        let app = index.with(Compression::default());

        for (algo, algo_name) in [
            (CompressionAlgo::BR, "br"),
            (CompressionAlgo::DEFLATE, "deflate"),
            (CompressionAlgo::GZIP, "gzip"),
        ] {
            let mut resp = app
                .call(
                    Request::builder()
                        .header(header::CONTENT_ENCODING, algo_name)
                        .header(header::ACCEPT_ENCODING, algo_name)
                        .body(compress_data(algo, DATA).await),
                )
                .await;
            let data = decompress_data(algo, &resp.take_body().into_vec().await.unwrap()).await;
            assert_eq!(data, DATA);
        }
    }
}
