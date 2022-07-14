use std::str::FromStr;

use typed_headers::{AcceptEncoding, ContentCoding, HeaderMapExt};

use crate::{
    http::header,
    web::{Compress, CompressionAlgo},
    Body, Endpoint, IntoResponse, Middleware, Request, Response, Result,
};

/// Middleware for decompress request body and compress response body.
///
/// It selects the decompression algorithm according to the request
/// `Content-Encoding` header, and selects the compression algorithm according
/// to the request `Accept-Encoding` header.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
#[derive(Default)]
pub struct Compression;

impl Compression {
    /// Creates a new `Compression` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<E: Endpoint> Middleware<E> for Compression {
    type Output = CompressionEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CompressionEndpoint { ep }
    }
}

/// Endpoint for Compression middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
pub struct CompressionEndpoint<E: Endpoint> {
    ep: E,
}

fn coding_priority(c: &ContentCoding) -> u8 {
    match *c {
        ContentCoding::DEFLATE => 1,
        ContentCoding::GZIP => 2,
        ContentCoding::BROTLI => 3,
        _ => 0,
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CompressionEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        // decompress request body
        if let Some(algo) = req
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| CompressionAlgo::from_str(value).ok())
        {
            let new_body = algo.decompress(req.take_body().into_async_read());
            req.set_body(Body::from_async_read(new_body));
        }

        // negotiate content-encoding
        let compress_algo = req
            .headers()
            .typed_get::<AcceptEncoding>()
            .ok()
            .flatten()
            .and_then(|encoding| {
                encoding
                    .0
                    .into_iter()
                    .max_by_key(|item| (item.quality, coding_priority(&item.item)))
            })
            .map(|c| c.item)
            .and_then(|coding| match coding {
                ContentCoding::GZIP => Some(CompressionAlgo::GZIP),
                ContentCoding::DEFLATE => Some(CompressionAlgo::DEFLATE),
                ContentCoding::STAR | ContentCoding::BROTLI => Some(CompressionAlgo::BR),
                _ => None,
            });

        match compress_algo {
            Some(algo) => Ok(Compress::new(self.ep.call(req).await?, algo).into_response()),
            None => Ok(self.ep.call(req).await?.into_response()),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::{handler, test::TestClient, EndpointExt};

    const DATA: &str = "abcdefghijklmnopqrstuvwxyz1234567890";
    const DATA_REV: &str = "0987654321zyxwvutsrqponmlkjihgfedcba";

    #[handler(internal)]
    async fn index(data: String) -> String {
        String::from_utf8(data.into_bytes().into_iter().rev().collect()).unwrap()
    }

    async fn test_algo(algo: CompressionAlgo) {
        let ep = index.with(Compression);
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Content-Encoding", algo.as_str())
            .header("Accept-Encoding", algo.as_str())
            .body(Body::from_async_read(algo.compress(DATA.as_bytes())))
            .send()
            .await;

        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", algo.as_str());

        let mut data = Vec::new();
        let mut reader = algo.decompress(resp.0.into_body().into_async_read());
        reader.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, DATA_REV.as_bytes());
    }

    #[tokio::test]
    async fn test_compression() {
        test_algo(CompressionAlgo::BR).await;
        test_algo(CompressionAlgo::DEFLATE).await;
        test_algo(CompressionAlgo::GZIP).await;
    }

    #[tokio::test]
    async fn test_negotiate() {
        let ep = index.with(Compression);
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Accept-Encoding", "identity; q=0.5, gzip;q=1.0, br;q=0.3")
            .body(DATA)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", "gzip");

        let mut data = Vec::new();
        let mut reader = CompressionAlgo::GZIP.decompress(resp.0.into_body().into_async_read());
        reader.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, DATA_REV.as_bytes());
    }

    #[tokio::test]
    async fn test_star() {
        let ep = index.with(Compression);
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Accept-Encoding", "identity; q=0.5, *;q=1.0, br;q=0.3")
            .body(DATA)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", "br");

        let mut data = Vec::new();
        let mut reader = CompressionAlgo::BR.decompress(resp.0.into_body().into_async_read());
        reader.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, DATA_REV.as_bytes());
    }

    #[tokio::test]
    async fn test_coding_priority() {
        let ep = index.with(Compression);
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(DATA)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", "br");

        let mut data = Vec::new();
        let mut reader = CompressionAlgo::BR.decompress(resp.0.into_body().into_async_read());
        reader.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, DATA_REV.as_bytes());
    }
}
