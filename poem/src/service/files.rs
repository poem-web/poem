use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use askama::Template;
use tokio::fs::File;

use crate::{
    http::{header, Method, StatusCode},
    Body, Endpoint, Request, Response,
};

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"
<html>
    <head>
        <title>Index of {{ path }}</title>
    </head>
    <body>
        <h1>Index of /{{ path }}</h1>
        <ul>
            {% for file in files %}
            <li>
                {% if file.is_dir %} 
                <a href="{{ file.url }}">{{ file.filename | e }}/</a>
                {% else %}
                <a href="{{ file.url }}">{{ file.filename | e }}</a>
                {% endif %}
            </li>
            {% endfor %}
        </ul>
    </body>
    </html>
"#
)]
struct DirectoryTemplate<'a> {
    path: &'a str,
    files: Vec<FileRef>,
}

struct FileRef {
    url: String,
    filename: String,
    is_dir: bool,
}

/// Static files handling service.
pub struct Files {
    path: PathBuf,
    show_files_listing: bool,
    index_file: Option<String>,
}

impl Files {
    /// Create new Files service for a specified base directory.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            show_files_listing: false,
            index_file: None,
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
}

#[async_trait::async_trait]
impl Endpoint for Files {
    type Output = Response;

    async fn call(&self, req: Request) -> Self::Output {
        if req.method() != Method::GET {
            return StatusCode::METHOD_NOT_ALLOWED.into();
        }

        let path = req
            .uri()
            .path()
            .trim_start_matches('/')
            .trim_end_matches('/');

        let path = match percent_encoding::percent_decode_str(path).decode_utf8() {
            Ok(path) => path,
            Err(_) => return StatusCode::BAD_REQUEST.into(),
        };

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
            return StatusCode::FORBIDDEN.into();
        }

        if !file_path.exists() {
            return StatusCode::NOT_FOUND.into();
        }

        if file_path.is_file() {
            create_file_response(&file_path).await
        } else {
            if let Some(index_file) = &self.index_file {
                let index_path = file_path.join(index_file);
                if index_path.is_file() {
                    return create_file_response(&index_path).await;
                }
            }

            if self.show_files_listing {
                let read_dir = match file_path.read_dir() {
                    Ok(d) => d,
                    Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into(),
                };
                let mut template = DirectoryTemplate {
                    path: &*path,
                    files: Vec::new(),
                };

                for res in read_dir {
                    let entry = match res {
                        Ok(entry) => entry,
                        Err(err) => {
                            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into()
                        }
                    };

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

                let html = match template.render() {
                    Ok(html) => html,
                    Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into(),
                };
                Response::builder()
                    .header(header::CONTENT_TYPE, mime::TEXT_HTML_UTF_8.as_ref())
                    .body(Body::from_string(html))
            } else {
                StatusCode::NOT_FOUND.into()
            }
        }
    }
}

async fn create_file_response(path: &Path) -> Response {
    let file = match File::open(path).await {
        Ok(file) => file,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into(),
    };
    Response::builder().body(Body::from_async_read(file))
}
