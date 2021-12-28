use std::{
    ffi::OsStr,
    fs::Metadata,
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use headers::{ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch, IfUnmodifiedSince};
use http::StatusCode;
use httpdate::HttpDate;
use mime::Mime;
use tokio::fs::File;

use crate::{
    error::StaticFileError,
    http::{header, Method},
    Body, Endpoint, Request, Response, Result,
};

struct DirectoryTemplate<'a> {
    path: &'a str,
    files: Vec<FileRef>,
}

impl<'a> DirectoryTemplate<'a> {
    fn render(&self) -> String {
        let mut s = format!(
            r#"
        <html>
            <head>
            <title>Index of {}</title>
        </head>
        <body>
        <h1>Index of /{}</h1>
        <ul>"#,
            self.path, self.path
        );

        for file in &self.files {
            if file.is_dir {
                s.push_str(&format!(
                    r#"<li><a href="{}">{}/</a></li>"#,
                    file.url, file.filename
                ));
            } else {
                s.push_str(&format!(
                    r#"<li><a href="{}">{}</a></li>"#,
                    file.url, file.filename
                ));
            }
        }

        s.push_str(
            r#"</ul>
        </body>
        </html>"#,
        );

        s
    }
}

struct FileRef {
    url: String,
    filename: String,
    is_dir: bool,
}

/// Static files handling service.
///
/// # Errors
///
/// - [`StaticFileError`]
#[cfg_attr(docsrs, doc(cfg(feature = "static-files")))]
pub struct StaticFiles {
    path: PathBuf,
    show_files_listing: bool,
    index_file: Option<String>,
    prefer_utf8: bool,
}

impl StaticFiles {
    /// Create new static files service for a specified base directory.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{endpoint::StaticFiles, Route};
    ///
    /// let app = Route::new().nest(
    ///     "/files",
    ///     StaticFiles::new("/etc/www")
    ///         .show_files_listing()
    ///         .index_file("index.html"),
    /// );
    /// ```
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            show_files_listing: false,
            index_file: None,
            prefer_utf8: true,
        }
    }

    /// Show files listing for directories.
    ///
    /// By default show files listing is disabled.
    #[must_use]
    pub fn show_files_listing(self) -> Self {
        Self {
            show_files_listing: true,
            ..self
        }
    }

    /// Set index file
    ///
    /// Shows specific index file for directories instead of showing files
    /// listing.
    ///
    /// If the index file is not found, files listing is shown as a fallback if
    /// Files::show_files_listing() is set.
    #[must_use]
    pub fn index_file(self, index: impl Into<String>) -> Self {
        Self {
            index_file: Some(index.into()),
            ..self
        }
    }

    /// Specifies whether text responses should signal a UTF-8 encoding.
    ///
    /// Default is `true`.
    #[must_use]
    pub fn prefer_utf8(self, value: bool) -> Self {
        Self {
            prefer_utf8: value,
            ..self
        }
    }
}

impl StaticFiles {
    async fn internal_call(&self, req: Request) -> Result<Response, StaticFileError> {
        if req.method() != Method::GET {
            return Err(StaticFileError::MethodNotAllowed(req.method().clone()));
        }

        let path = req
            .uri()
            .path()
            .trim_start_matches('/')
            .trim_end_matches('/');

        let path = percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .map_err(|_| StaticFileError::InvalidPath)?;

        let mut file_path = self.path.clone();
        for p in Path::new(&*path) {
            if p == OsStr::new(".") {
                continue;
            } else if p == OsStr::new("..") {
                file_path.pop();
            } else {
                file_path.push(&p);
            }
        }

        if !file_path.starts_with(&self.path) {
            return Err(StaticFileError::Forbidden(file_path.display().to_string()));
        }

        if !file_path.exists() {
            return Err(StaticFileError::NotFound(file_path.display().to_string()));
        }

        if file_path.is_file() {
            create_file_response(&file_path, &req, self.prefer_utf8).await
        } else {
            if let Some(index_file) = &self.index_file {
                let index_path = file_path.join(index_file);
                if index_path.is_file() {
                    return create_file_response(&index_path, &req, self.prefer_utf8).await;
                }
            }

            if self.show_files_listing {
                let read_dir = file_path.read_dir()?;
                let mut template = DirectoryTemplate {
                    path: &*path,
                    files: Vec::new(),
                };

                for res in read_dir {
                    let entry = res?;

                    if let Some(filename) = entry.file_name().to_str() {
                        let mut base_url = req.original_uri().path().to_string();
                        if !base_url.ends_with('/') {
                            base_url.push('/');
                        }
                        template.files.push(FileRef {
                            url: format!("{}{}", base_url, filename),
                            filename: filename.to_string(),
                            is_dir: entry.path().is_dir(),
                        });
                    }
                }

                let html = template.render();
                Ok(Response::builder()
                    .header(header::CONTENT_TYPE, mime::TEXT_HTML_UTF_8.as_ref())
                    .body(Body::from_string(html)))
            } else {
                Err(StaticFileError::NotFound(file_path.display().to_string()))
            }
        }
    }
}

#[async_trait::async_trait]
impl Endpoint for StaticFiles {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.internal_call(req).await.map_err(Into::into)
    }
}

/// Single static file handling service.
///
/// # Errors
///
/// - [`StaticFileError`]
#[cfg_attr(docsrs, doc(cfg(feature = "static-files")))]
pub struct StaticFile {
    path: PathBuf,
    prefer_utf8: bool,
}

impl StaticFile {
    /// Create new single static file service for a specified file path.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{endpoint::StaticFile, Route};
    ///
    /// let app = Route::new().at("/logo.png", StaticFile::new("/etc/logo.png"));
    /// ```
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            prefer_utf8: true,
        }
    }

    /// Specifies whether text responses should signal a UTF-8 encoding.
    ///
    /// Default is `true`.
    #[must_use]
    pub fn prefer_utf8(self, value: bool) -> Self {
        Self {
            prefer_utf8: value,
            ..self
        }
    }
}

#[async_trait::async_trait]
impl Endpoint for StaticFile {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        Ok(create_file_response(&self.path, &req, self.prefer_utf8).await?)
    }
}

async fn create_file_response(
    path: &Path,
    req: &Request,
    prefer_utf8: bool,
) -> Result<Response, StaticFileError> {
    let guess = mime_guess::from_path(path);
    let file = File::open(path).await?;
    let metadata = file.metadata().await?;
    let mut builder = Response::builder();

    // content type
    if let Some(mut mime) = guess.first() {
        if prefer_utf8 {
            mime = equiv_utf8_text(mime);
        }
        builder = builder.header(header::CONTENT_TYPE, mime.to_string());
    }

    if let Ok(modified) = metadata.modified() {
        let etag = etag(ino(&metadata), &modified, metadata.len());

        if let Some(if_match) = req.headers().typed_get::<IfMatch>() {
            if !if_match.precondition_passes(&etag) {
                return Ok(builder.status(StatusCode::PRECONDITION_FAILED).finish());
            }
        }

        if let Some(if_unmodified_since) = req.headers().typed_get::<IfUnmodifiedSince>() {
            if !if_unmodified_since.precondition_passes(modified) {
                return Ok(builder.status(StatusCode::PRECONDITION_FAILED).finish());
            }
        }

        if let Some(if_non_match) = req.headers().typed_get::<IfNoneMatch>() {
            if !if_non_match.precondition_passes(&etag) {
                return Ok(builder.status(StatusCode::NOT_MODIFIED).finish());
            }
        } else if let Some(if_modified_since) = req.headers().typed_get::<IfModifiedSince>() {
            if !if_modified_since.is_modified(modified) {
                return Ok(builder.status(StatusCode::NOT_MODIFIED).finish());
            }
        }

        builder = builder
            .header(header::CACHE_CONTROL, "public")
            .header(header::LAST_MODIFIED, HttpDate::from(modified).to_string());
        builder = builder.typed_header(etag);
    }

    Ok(builder.body(Body::from_async_read(file)))
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

fn etag(ino: u64, modified: &SystemTime, len: u64) -> ETag {
    let dur = modified
        .duration_since(UNIX_EPOCH)
        .expect("modification time must be after epoch");

    ETag::from_str(&format!(
        "\"{:x}:{:x}:{:x}:{:x}\"",
        ino,
        len,
        dur.as_secs(),
        dur.subsec_nanos()
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

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

    #[tokio::test]
    async fn test_if_none_match() {
        let resp = create_file_response(Path::new("Cargo.toml"), &Request::default(), false)
            .await
            .unwrap();
        assert!(resp.is_ok());
        let etag = resp.header("etag").unwrap();

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder().header("if-none-match", etag).finish(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder().header("if-none-match", "abc").finish(),
            false,
        )
        .await
        .unwrap();
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn test_if_modified_since() {
        let resp = create_file_response(Path::new("Cargo.toml"), &Request::default(), false)
            .await
            .unwrap();
        assert!(resp.is_ok());
        let modified = resp.header("last-modified").unwrap();

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-modified-since", modified)
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);

        let mut t: SystemTime = HttpDate::from_str(modified).unwrap().into();
        t -= Duration::from_secs(1);

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert!(resp.is_ok());

        let mut t: SystemTime = HttpDate::from_str(modified).unwrap().into();
        t += Duration::from_secs(1);

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-modified-since", HttpDate::from(t).to_string())
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn test_if_match() {
        let resp = create_file_response(Path::new("Cargo.toml"), &Request::default(), false)
            .await
            .unwrap();
        assert!(resp.is_ok());
        let etag = resp.header("etag").unwrap();

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder().header("if-match", etag).finish(),
            false,
        )
        .await
        .unwrap();
        assert!(resp.is_ok());

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder().header("if-match", "abc").finish(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::PRECONDITION_FAILED);
    }

    #[tokio::test]
    async fn test_if_unmodified_since() {
        let resp = create_file_response(Path::new("Cargo.toml"), &Request::default(), false)
            .await
            .unwrap();
        assert!(resp.is_ok());
        let modified = resp.header("last-modified").unwrap();

        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-unmodified-since", modified)
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert!(resp.is_ok());

        let mut t: SystemTime = HttpDate::from_str(modified).unwrap().into();
        t += Duration::from_secs(1);
        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert!(resp.is_ok());

        let mut t: SystemTime = HttpDate::from_str(modified).unwrap().into();
        t -= Duration::from_secs(1);
        let resp = create_file_response(
            Path::new("Cargo.toml"),
            &Request::builder()
                .header("if-unmodified-since", HttpDate::from(t).to_string())
                .finish(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::PRECONDITION_FAILED);
    }
}
