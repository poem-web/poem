use std::marker::PhantomData;

use rust_embed::RustEmbed;

use crate::{
    Endpoint, Error, Request, Response, Result,
    error::StaticFileError,
    http::{Method, StatusCode, header},
};

/// An endpoint that wraps a single file from a `rust-embed` bundle.
pub struct EmbeddedFileEndpoint<E: RustEmbed + Send + Sync> {
    _embed: PhantomData<E>,
    path: String,
}

impl<E: RustEmbed + Send + Sync> EmbeddedFileEndpoint<E> {
    /// Create a new `EmbeddedFileEndpoint` from a `rust-embed` bundle.
    ///
    /// `path` - relative path within the bundle.
    ///
    pub fn new(path: &str) -> Self {
        EmbeddedFileEndpoint {
            _embed: PhantomData,
            path: path.to_owned(),
        }
    }
}

impl<E: RustEmbed + Send + Sync> Endpoint for EmbeddedFileEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() != Method::GET {
            return Err(StaticFileError::MethodNotAllowed(req.method().clone()).into());
        }

        match E::get(&self.path) {
            Some(content) => {
                let hash = hex::encode(content.metadata.sha256_hash());
                if req
                    .headers()
                    .get(header::IF_NONE_MATCH)
                    .map(|etag| etag.to_str().unwrap_or("000000").eq(&hash))
                    .unwrap_or(false)
                {
                    return Err(StatusCode::NOT_MODIFIED.into());
                }

                // otherwise, return 200 with etag hash
                let body: Vec<u8> = content.data.into();
                let mime = mime_guess::from_path(&self.path).first_or_octet_stream();
                Ok(Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .header(header::ETAG, hash)
                    .body(body))
            }
            None => Err(StatusCode::NOT_FOUND.into()),
        }
    }
}

/// An endpoint that wraps a `rust-embed` bundle.
///
/// # Errors
///
/// - [`StaticFileError`]
#[cfg_attr(docsrs, doc(cfg(feature = "embed")))]
pub struct EmbeddedFilesEndpoint<E: RustEmbed + Send + Sync> {
    _embed: PhantomData<E>,
    index_file: Option<String>,
}

impl<E: RustEmbed + Sync + Send> Default for EmbeddedFilesEndpoint<E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E: RustEmbed + Send + Sync> EmbeddedFilesEndpoint<E> {
    /// Create a new `EmbeddedFilesEndpoint` from a `rust-embed` bundle.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{Route, endpoint::EmbeddedFilesEndpoint};
    ///
    /// #[derive(RustEmbed)]
    /// #[folder = "/etc/www"]
    /// pub struct Files;
    ///
    /// let app = Route::new().nest(
    ///     "/files",
    ///     EmbeddedFilesEndpoint::<Files>::new()
    ///         .index_file("index.html"),
    /// );
    /// ```
    pub fn new() -> Self {
        EmbeddedFilesEndpoint {
            _embed: PhantomData,
            index_file: None,
        }
    }

    /// Set index file
    ///
    /// Shows specific index file for directories instead of showing files
    /// listing.
    pub fn index_file(self, index: impl Into<String>) -> Self {
        Self {
            index_file: Some(index.into()),
            ..self
        }
    }
}

impl<E: RustEmbed + Send + Sync> Endpoint for EmbeddedFilesEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let path = req.uri().path().trim_start_matches('/');
        let original_path = req.original_uri().path();
        let original_end_with_slash = original_path.ends_with('/');

        use header::LOCATION;

        if path.is_empty() && !original_end_with_slash {
            return Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header(LOCATION, format!("{original_path}/"))
                .finish());
        };

        if original_end_with_slash && E::get(&format!("{path}index.html")).is_some() {
            let path = format!("{path}index.html");
            EmbeddedFileEndpoint::<E>::new(&path).call(req).await
        } else if E::get(path).is_some() {
            EmbeddedFileEndpoint::<E>::new(path).call(req).await
        } else if E::get(&format!("{path}/index.html")).is_some() {
            Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header(LOCATION, format!("{original_path}/"))
                .finish())
        } else if let Some(index_file) = &self.index_file {
            EmbeddedFileEndpoint::<E>::new(index_file).call(req).await
        } else {
            EmbeddedFileEndpoint::<E>::new(path).call(req).await
        }
    }
}
