use std::{collections::HashSet, str::FromStr};

use headers::HeaderMap;

use crate::{
    http::header,
    web::{Compress, CompressionAlgo, CompressionLevel},
    Body, Endpoint, IntoResponse, Middleware, Request, Response, Result,
};

enum ContentCoding {
    Brotli,
    Deflate,
    Gzip,
    Star,
}

impl FromStr for ContentCoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("deflate") {
            Ok(ContentCoding::Deflate)
        } else if s.eq_ignore_ascii_case("gzip") {
            Ok(ContentCoding::Gzip)
        } else if s.eq_ignore_ascii_case("br") {
            Ok(ContentCoding::Brotli)
        } else if s == "*" {
            Ok(ContentCoding::Star)
        } else {
            Err(())
        }
    }
}

fn parse_accept_encoding(
    headers: &HeaderMap,
    enabled_algorithms: &HashSet<CompressionAlgo>,
) -> Option<ContentCoding> {
    headers
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|hval| hval.to_str().ok())
        .flat_map(|s| s.split(',').map(str::trim))
        .filter_map(|v| {
            let (e, q) = match v.split_once(";q=") {
                Some((e, q)) => (e, (q.parse::<f32>().ok()? * 1000.0) as i32),
                None => (v, 1000),
            };
            let coding: ContentCoding = e.parse().ok()?;
            Some((coding, q))
        })
        .filter(|(encoding, _)| {
            if !enabled_algorithms.is_empty() {
                match encoding {
                    ContentCoding::Brotli => enabled_algorithms.contains(&CompressionAlgo::BR),
                    ContentCoding::Deflate => {
                        enabled_algorithms.contains(&CompressionAlgo::DEFLATE)
                    }
                    ContentCoding::Gzip => enabled_algorithms.contains(&CompressionAlgo::GZIP),
                    _ => true,
                }
            } else {
                true
            }
        })
        .max_by_key(|(coding, q)| (*q, coding_priority(coding)))
        .map(|(coding, _)| coding)
}

/// Middleware for decompress request body and compress response body.
///
/// It selects the decompression algorithm according to the request
/// `Content-Encoding` header, and selects the compression algorithm according
/// to the request `Accept-Encoding` header.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
#[derive(Default)]
pub struct Compression {
    level: Option<CompressionLevel>,
    algorithms: HashSet<CompressionAlgo>,
}

impl Compression {
    /// Creates a new `Compression` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Specify the compression level
    #[must_use]
    #[inline]
    pub fn with_quality(self, level: CompressionLevel) -> Self {
        Self {
            level: Some(level),
            ..self
        }
    }

    /// Specify the enabled algorithms (default to all)
    #[must_use]
    #[inline]
    pub fn algorithms(self, algorithms: impl IntoIterator<Item = CompressionAlgo>) -> Self {
        Self {
            algorithms: algorithms.into_iter().collect(),
            ..self
        }
    }
}

impl<E: Endpoint> Middleware<E> for Compression {
    type Output = CompressionEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CompressionEndpoint {
            ep,
            level: self.level,
            algorithms: self.algorithms.clone(),
        }
    }
}

/// Endpoint for Compression middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
pub struct CompressionEndpoint<E: Endpoint> {
    ep: E,
    level: Option<CompressionLevel>,
    algorithms: HashSet<CompressionAlgo>,
}

#[inline]
fn coding_priority(c: &ContentCoding) -> u8 {
    match *c {
        ContentCoding::Deflate => 1,
        ContentCoding::Gzip => 2,
        ContentCoding::Brotli => 3,
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
        let compress_algo =
            parse_accept_encoding(req.headers(), &self.algorithms).map(|coding| match coding {
                ContentCoding::Gzip => CompressionAlgo::GZIP,
                ContentCoding::Deflate => CompressionAlgo::DEFLATE,
                ContentCoding::Star | ContentCoding::Brotli => CompressionAlgo::BR,
            });

        let resp = self.ep.call(req).await?;
        match compress_algo {
            Some(algo) => {
                let mut compress = Compress::new(resp, algo);
                if let Some(level) = self.level {
                    compress = compress.with_quality(level);
                }
                Ok(compress.into_response())
            }
            None => Ok(resp.into_response()),
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
        let ep = index.with(Compression::default());
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Content-Encoding", algo.as_str())
            .header("Accept-Encoding", algo.as_str())
            .body(Body::from_async_read(algo.compress(DATA.as_bytes(), None)))
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
        let ep = index.with(Compression::default());
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
        let ep = index.with(Compression::default());
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
        let ep = index.with(Compression::default());
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

    #[tokio::test]
    async fn test_enabled_algorithms() {
        let ep = index.with(Compression::default().algorithms([CompressionAlgo::GZIP]));
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(DATA)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", "gzip");

        let ep = index.with(Compression::default().algorithms([CompressionAlgo::BR]));
        let cli = TestClient::new(ep);

        let resp = cli
            .post("/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(DATA)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header("Content-Encoding", "br");
    }
}
