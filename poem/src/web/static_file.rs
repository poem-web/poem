use std::{
    fs::Metadata,
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use headers::{ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch, IfUnmodifiedSince};
use http::{header, StatusCode};
use httpdate::HttpDate;
use mime::Mime;
use tokio::fs::File;

use crate::{
    error::StaticFileError, Body, FromRequest, IntoResponse, Request, RequestBody, Response, Result,
};

/// A response for static file extractor.
pub enum StaticFileResponse {
    /// 200 OK
    Ok {
        /// Response body
        body: Body,
        /// Content type
        content_type: Option<String>,
        /// `ETag` header value
        etag: Option<String>,
        /// `Last-Modified` header value
        last_modified: Option<String>,
    },
    /// 412 PRECONDITION_FAILED
    PreconditionFailed,
    /// 304 NOT_MODIFIED
    NotModified,
}

impl IntoResponse for StaticFileResponse {
    fn into_response(self) -> Response {
        match self {
            StaticFileResponse::Ok {
                body,
                content_type,
                etag,
                last_modified,
            } => {
                let mut builder = Response::builder();
                if let Some(content_type) = content_type {
                    builder = builder.content_type(&content_type);
                }
                if let Some(etag) = etag {
                    builder = builder.header(header::ETAG, etag);
                }
                if let Some(last_modified) = last_modified {
                    builder = builder.header(header::LAST_MODIFIED, last_modified);
                }
                builder.body(body)
            }
            StaticFileResponse::PreconditionFailed => StatusCode::PRECONDITION_FAILED.into(),
            StaticFileResponse::NotModified => StatusCode::NOT_MODIFIED.into(),
        }
    }
}

/// An extractor for responding static files.
pub struct StaticFileRequest {
    if_match: Option<IfMatch>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_none_match: Option<IfNoneMatch>,
    if_modified_since: Option<IfModifiedSince>,
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for StaticFileRequest {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(Self {
            if_match: req.headers().typed_get::<IfMatch>(),
            if_unmodified_since: req.headers().typed_get::<IfUnmodifiedSince>(),
            if_none_match: req.headers().typed_get::<IfNoneMatch>(),
            if_modified_since: req.headers().typed_get::<IfModifiedSince>(),
        })
    }
}

impl StaticFileRequest {
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
        let guess = mime_guess::from_path(path);
        let file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;

        // content type
        let content_type = guess.first().map(|mime| {
            if prefer_utf8 {
                equiv_utf8_text(mime).to_string()
            } else {
                mime.to_string()
            }
        });

        let mut etag_str = String::new();
        let mut last_modified_str = String::new();

        if let Ok(modified) = metadata.modified() {
            etag_str = etag(ino(&metadata), &modified, metadata.len());
            let etag = ETag::from_str(&etag_str).unwrap();

            if let Some(if_match) = self.if_match {
                if !if_match.precondition_passes(&etag) {
                    return Ok(StaticFileResponse::PreconditionFailed);
                }
            }

            if let Some(if_unmodified_since) = self.if_unmodified_since {
                if !if_unmodified_since.precondition_passes(modified) {
                    return Ok(StaticFileResponse::PreconditionFailed);
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

        Ok(StaticFileResponse::Ok {
            body: Body::from_async_read(File::from_std(file)),
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

    async fn check_response(req: Request) -> StaticFileResponse {
        let static_file = StaticFileRequest::from_request_without_body(&req)
            .await
            .unwrap();
        static_file
            .create_response(Path::new("Cargo.toml"), false)
            .unwrap()
    }

    #[tokio::test]
    async fn test_if_none_match() {
        let resp = check_response(Request::default()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let etag = resp.etag();

        let resp = check_response(Request::builder().header("if-none-match", etag).finish()).await;
        assert!(matches!(resp, StaticFileResponse::NotModified));

        let resp = check_response(Request::builder().header("if-none-match", "abc").finish()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
    }

    #[tokio::test]
    async fn test_if_modified_since() {
        let resp = check_response(Request::default()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let modified = resp.last_modified();

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", &modified)
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::NotModified));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t -= Duration::from_secs(1);

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t += Duration::from_secs(1);

        let resp = check_response(
            Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::NotModified));
    }

    #[tokio::test]
    async fn test_if_match() {
        let resp = check_response(Request::default()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let etag = resp.etag();

        let resp = check_response(Request::builder().header("if-match", etag).finish()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let resp = check_response(Request::builder().header("if-match", "abc").finish()).await;
        assert!(matches!(resp, StaticFileResponse::PreconditionFailed));
    }

    #[tokio::test]
    async fn test_if_unmodified_since() {
        let resp = check_response(Request::default()).await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));
        let modified = resp.last_modified();

        let resp = check_response(
            Request::builder()
                .header("if-unmodified-since", &modified)
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t += Duration::from_secs(1);
        let resp = check_response(
            Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::Ok { .. }));

        let mut t: SystemTime = HttpDate::from_str(&modified).unwrap().into();
        t -= Duration::from_secs(1);
        let resp = check_response(
            Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
        )
        .await;
        assert!(matches!(resp, StaticFileResponse::PreconditionFailed));
    }
}
