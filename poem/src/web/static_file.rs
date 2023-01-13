use std::{
    collections::Bound,
    fs::Metadata,
    io::{Seek, SeekFrom},
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use headers::{
    ContentRange, ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch, IfUnmodifiedSince,
    Range,
};
use http::{header, StatusCode};
use httpdate::HttpDate;
use mime::Mime;
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    error::StaticFileError, Body, FromRequest, IntoResponse, Request, RequestBody, Response, Result,
};

/// A response for static file extractor.
#[derive(Debug)]
pub enum StaticFileResponse {
    /// 200 OK
    Ok {
        /// Response body
        body: Body,
        /// Content length
        content_length: u64,
        /// Content type
        content_type: Option<String>,
        /// `ETag` header value
        etag: Option<String>,
        /// `Last-Modified` header value
        last_modified: Option<String>,
        /// `Content-Range` header value
        content_range: Option<(std::ops::Range<u64>, u64)>,
    },
    /// 304 NOT MODIFIED
    NotModified,
}

impl StaticFileResponse {
    /// Set the content type
    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        if let StaticFileResponse::Ok { content_type, .. } = &mut self {
            *content_type = Some(ct.into());
        }
        self
    }
}

impl IntoResponse for StaticFileResponse {
    fn into_response(self) -> Response {
        match self {
            StaticFileResponse::Ok {
                body,
                content_length,
                content_type,
                etag,
                last_modified,
                content_range,
            } => {
                let mut builder = Response::builder()
                    .header(header::ACCEPT_RANGES, "bytes")
                    .header(header::CONTENT_LENGTH, content_length);

                if let Some(content_type) = content_type {
                    builder = builder.content_type(content_type);
                }
                if let Some(etag) = etag {
                    builder = builder.header(header::ETAG, etag);
                }
                if let Some(last_modified) = last_modified {
                    builder = builder.header(header::LAST_MODIFIED, last_modified);
                }

                if let Some((range, size)) = content_range {
                    builder = builder
                        .status(StatusCode::PARTIAL_CONTENT)
                        .typed_header(ContentRange::bytes(range, size).unwrap());
                }

                builder.body(body)
            }
            StaticFileResponse::NotModified => StatusCode::NOT_MODIFIED.into(),
        }
    }
}

/// An extractor for responding static files.
#[derive(Debug)]
pub struct StaticFileRequest {
    if_match: Option<IfMatch>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_none_match: Option<IfNoneMatch>,
    if_modified_since: Option<IfModifiedSince>,
    range: Option<Range>,
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for StaticFileRequest {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(Self {
            if_match: req.headers().typed_get::<IfMatch>(),
            if_unmodified_since: req.headers().typed_get::<IfUnmodifiedSince>(),
            if_none_match: req.headers().typed_get::<IfNoneMatch>(),
            if_modified_since: req.headers().typed_get::<IfModifiedSince>(),
            range: req.headers().typed_get::<Range>(),
        })
    }
}

impl StaticFileRequest {
    /// Create static file response.
    ///
    /// `prefer_utf8` - Specifies whether text responses should signal a UTF-8
    /// encoding.
    pub fn create_response_from_data(
        self,
        data: impl AsRef<[u8]>,
    ) -> Result<StaticFileResponse, StaticFileError> {
        let data = data.as_ref();

        // content length
        let mut content_length = data.len() as u64;
        let mut content_range = None;

        let body = if let Some((start, end)) = self.range.and_then(|range| range.iter().next()) {
            let start = match start {
                Bound::Included(n) => n,
                Bound::Excluded(n) => n + 1,
                Bound::Unbounded => 0,
            };
            let end = match end {
                Bound::Included(n) => n + 1,
                Bound::Excluded(n) => n,
                Bound::Unbounded => content_length,
            };
            if end < start || end > content_length {
                return Err(StaticFileError::RangeNotSatisfiable {
                    size: content_length,
                });
            }

            if start != 0 || end != content_length {
                content_range = Some((start..end, content_length));
            }

            content_length = end - start;
            Body::from_bytes(Bytes::copy_from_slice(
                &data[start as usize..(start + content_length) as usize],
            ))
        } else {
            Body::from_bytes(Bytes::copy_from_slice(data))
        };

        Ok(StaticFileResponse::Ok {
            body,
            content_length,
            content_type: None,
            etag: None,
            last_modified: None,
            content_range,
        })
    }

    /// Create static file response.
    ///
    /// `prefer_utf8` - Specifies whether text responses should signal a UTF-8
    /// encoding.
    pub fn create_response(
        self,
        path: impl AsRef<Path>,
        prefer_utf8: bool,
    ) -> Result<StaticFileResponse, StaticFileError> {
        let path = path.as_ref();
        if !path.exists() || !path.is_file() {
            return Err(StaticFileError::NotFound);
        }
        let guess = mime_guess::from_path(path);
        let mut file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;

        // content length
        let mut content_length = metadata.len();

        // content type
        let content_type = guess.first().map(|mime| {
            if prefer_utf8 {
                equiv_utf8_text(mime).to_string()
            } else {
                mime.to_string()
            }
        });

        // etag and last modified
        let mut etag_str = String::new();
        let mut last_modified_str = String::new();

        if let Ok(modified) = metadata.modified() {
            etag_str = etag(ino(&metadata), &modified, metadata.len());
            let etag = ETag::from_str(&etag_str).unwrap();

            if let Some(if_match) = self.if_match {
                if !if_match.precondition_passes(&etag) {
                    return Err(StaticFileError::PreconditionFailed);
                }
            }

            if let Some(if_unmodified_since) = self.if_unmodified_since {
                if !if_unmodified_since.precondition_passes(modified) {
                    return Err(StaticFileError::PreconditionFailed);
                }
            }

            if let Some(if_non_match) = self.if_none_match {
                if !if_non_match.precondition_passes(&etag) {
                    return Ok(StaticFileResponse::NotModified);
                }
            } else if let Some(if_modified_since) = self.if_modified_since {
                if !if_modified_since.is_modified(modified) {
                    return Ok(StaticFileResponse::NotModified);
                }
            }

            last_modified_str = HttpDate::from(modified).to_string();
        }

        let mut content_range = None;

        let body = if let Some((start, end)) = self.range.and_then(|range| range.iter().next()) {
            let start = match start {
                Bound::Included(n) => n,
                Bound::Excluded(n) => n + 1,
                Bound::Unbounded => 0,
            };
            let end = match end {
                Bound::Included(n) => n + 1,
                Bound::Excluded(n) => n,
                Bound::Unbounded => metadata.len(),
            };
            if end < start || end > metadata.len() {
                return Err(StaticFileError::RangeNotSatisfiable {
                    size: metadata.len(),
                });
            }

            if start != 0 || end != metadata.len() {
                content_range = Some((start..end, metadata.len()));
            }

            content_length = end - start;
            file.seek(SeekFrom::Start(start))?;
            Body::from_async_read(File::from_std(file).take(end - start))
        } else {
            Body::from_async_read(File::from_std(file))
        };

        Ok(StaticFileResponse::Ok {
            body,
            content_length,
            content_type,
            etag: if !etag_str.is_empty() {
                Some(etag_str)
            } else {
                None
            },
            last_modified: if !last_modified_str.is_empty() {
                Some(last_modified_str)
            } else {
                None
            },
            content_range,
        })
    }
}

fn equiv_utf8_text(ct: Mime) -> Mime {
    if ct == mime::APPLICATION_JAVASCRIPT {
        return mime::APPLICATION_JAVASCRIPT_UTF_8;
    }

    if ct == mime::TEXT_HTML {
        return mime::TEXT_HTML_UTF_8;
    }

    if ct == mime::TEXT_CSS {
        return mime::TEXT_CSS_UTF_8;
    }

    if ct == mime::TEXT_PLAIN {
        return mime::TEXT_PLAIN_UTF_8;
    }

    if ct == mime::TEXT_CSV {
        return mime::TEXT_CSV_UTF_8;
    }

    if ct == mime::TEXT_TAB_SEPARATED_VALUES {
        return mime::TEXT_TAB_SEPARATED_VALUES_UTF_8;
    }

    ct
}

#[allow(unused_variables)]
fn ino(md: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::ino(md)
    }
    #[cfg(not(unix))]
    {
        0
    }
}

fn etag(ino: u64, modified: &SystemTime, len: u64) -> String {
    let dur = modified
        .duration_since(UNIX_EPOCH)
        .expect("modification time must be after epoch");

    format!(
        "\"{:x}:{:x}:{:x}:{:x}\"",
        ino,
        len,
        dur.as_secs(),
        dur.subsec_nanos()
    )
}

#[cfg(test)]
mod tests {
    use std::{path::Path, time::Duration};

    use super::*;

    impl StaticFileResponse {
        fn etag(&self) -> String {
            match self {
                StaticFileResponse::Ok { etag, .. } => etag.clone().unwrap(),
                _ => panic!(),
            }
        }

        fn last_modified(&self) -> String {
            match self {
                StaticFileResponse::Ok { last_modified, .. } => last_modified.clone().unwrap(),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn test_equiv_utf8_text() {
        assert_eq!(
            equiv_utf8_text(mime::APPLICATION_JAVASCRIPT),
            mime::APPLICATION_JAVASCRIPT_UTF_8
        );
        assert_eq!(equiv_utf8_text(mime::TEXT_HTML), mime::TEXT_HTML_UTF_8);
        assert_eq!(equiv_utf8_text(mime::TEXT_CSS), mime::TEXT_CSS_UTF_8);
        assert_eq!(equiv_utf8_text(mime::TEXT_PLAIN), mime::TEXT_PLAIN_UTF_8);
        assert_eq!(equiv_utf8_text(mime::TEXT_CSV), mime::TEXT_CSV_UTF_8);
        assert_eq!(
            equiv_utf8_text(mime::TEXT_TAB_SEPARATED_VALUES),
            mime::TEXT_TAB_SEPARATED_VALUES_UTF_8
        );

        assert_eq!(equiv_utf8_text(mime::TEXT_XML), mime::TEXT_XML);
        assert_eq!(equiv_utf8_text(mime::IMAGE_PNG), mime::IMAGE_PNG);
    }

    async fn check_response(req: Request) -> Result<StaticFileResponse, StaticFileError> {
        let static_file = StaticFileRequest::from_request_without_body(&req)
            .await
            .unwrap();
        static_file.create_response(Path::new("Cargo.toml"), false)
    }

    #[tokio::test]
    async fn test_if_none_match() {
        let resp = check_response(Request::default()).await.unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let etag = resp.etag();

        let resp = check_response(Request::builder().header("if-none-match", etag).finish())
            .await
            .unwrap();
        assert!(matches!(resp, StaticFileResponse::NotModified));

        let resp = check_response(Request::builder().header("if-none-match", "abc").finish())
            .await
            .unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
    }

    #[tokio::test]
    async fn test_if_modified_since() {
        let resp = check_response(Request::default()).await.unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let modified = resp.last_modified();

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", &modified)
                .finish(),
        )
        .await
        .unwrap();
        assert!(matches!(resp, StaticFileResponse::NotModified));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t -= Duration::from_secs(1);

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await
        .unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t += Duration::from_secs(1);

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await
        .unwrap();
        assert!(matches!(resp, StaticFileResponse::NotModified));
    }

    #[tokio::test]
    async fn test_if_match() {
        let resp = check_response(Request::default()).await.unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let etag = resp.etag();

        let resp = check_response(Request::builder().header("if-match", etag).finish())
            .await
            .unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let err = check_response(Request::builder().header("if-match", "abc").finish())
            .await
            .unwrap_err();
        assert!(matches!(err, StaticFileError::PreconditionFailed));
    }

    #[tokio::test]
    async fn test_if_unmodified_since() {
        let resp = check_response(Request::default()).await.unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let modified = resp.last_modified();

        let resp = check_response(
            Request::builder()
                .header("if-unmodified-since", &modified)
                .finish(),
        )
        .await
        .unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t += Duration::from_secs(1);
        let resp = check_response(
            Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await
        .unwrap();
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t -= Duration::from_secs(1);
        let err = check_response(
            Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, StaticFileError::PreconditionFailed));
    }

    #[tokio::test]
    async fn test_range_partial_content() {
        let static_file = StaticFileRequest::from_request_without_body(
            &Request::builder()
                .typed_header(Range::bytes(0..10).unwrap())
                .finish(),
        )
        .await
        .unwrap();
        let resp = static_file
            .create_response(Path::new("Cargo.toml"), false)
            .unwrap();
        match resp {
            StaticFileResponse::Ok { content_range, .. } => {
                assert_eq!(content_range.unwrap().0, 0..10);
            }
            StaticFileResponse::NotModified => panic!(),
        }
    }

    #[tokio::test]
    async fn test_range_full_content() {
        let md = std::fs::metadata("Cargo.toml").unwrap();

        let static_file = StaticFileRequest::from_request_without_body(
            &Request::builder()
                .typed_header(Range::bytes(0..md.len()).unwrap())
                .finish(),
        )
        .await
        .unwrap();
        let resp = static_file
            .create_response(Path::new("Cargo.toml"), false)
            .unwrap();
        match resp {
            StaticFileResponse::Ok { content_range, .. } => {
                assert!(content_range.is_none());
            }
            StaticFileResponse::NotModified => panic!(),
        }
    }

    #[tokio::test]
    async fn test_range_413() {
        let md = std::fs::metadata("Cargo.toml").unwrap();

        let static_file = StaticFileRequest::from_request_without_body(
            &Request::builder()
                .typed_header(Range::bytes(0..md.len() + 1).unwrap())
                .finish(),
        )
        .await
        .unwrap();
        let err = static_file
            .create_response(Path::new("Cargo.toml"), false)
            .unwrap_err();

        match err {
            StaticFileError::RangeNotSatisfiable { size } => assert_eq!(size, md.len()),
            _ => panic!(),
        }
    }
}
