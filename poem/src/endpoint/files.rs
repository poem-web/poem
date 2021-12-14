use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use mime::Mime;
use tokio::fs::File;

use crate::{
    error::StaticFileError,
    http::{header, HeaderValue, Method},
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
pub struct Files {
    path: PathBuf,
    show_files_listing: bool,
    index_file: Option<String>,
    prefer_utf8: bool,
}

impl Files {
    /// Create new Files service for a specified base directory.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{endpoint::Files, Route};
    ///
    /// let app = Route::new().nest(
    ///     "/files",
    ///     Files::new("/etc/www")
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
    pub fn index_file(self, index: impl Into<String>) -> Self {
        Self {
            index_file: Some(index.into()),
            ..self
        }
    }

    /// Specifies whether text responses should signal a UTF-8 encoding.
    ///
    /// Default is `true`.
    pub fn prefer_utf8(self, value: bool) -> Self {
        Self {
            prefer_utf8: value,
            ..self
        }
    }
}

impl Files {
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
            create_file_response(&file_path, self.prefer_utf8).await
        } else {
            if let Some(index_file) = &self.index_file {
                let index_path = file_path.join(index_file);
                if index_path.is_file() {
                    return create_file_response(&index_path, self.prefer_utf8).await;
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
impl Endpoint for Files {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.internal_call(req).await.map_err(Into::into)
    }
}

async fn create_file_response(path: &Path, prefer_utf8: bool) -> Result<Response, StaticFileError> {
    let guess = mime_guess::from_path(path);
    let file = File::open(path).await?;
    let mut resp = Response::builder().body(Body::from_async_read(file));
    if let Some(mut mime) = guess.first() {
        if prefer_utf8 {
            mime = equiv_utf8_text(mime);
        }
        if let Ok(header_value) = HeaderValue::from_str(&mime.to_string()) {
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, header_value);
        }
    }
    Ok(resp)
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

#[cfg(test)]
mod tests {
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
}
