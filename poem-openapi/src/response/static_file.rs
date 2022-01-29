use poem::{error::StaticFileError, Body};

use crate::{
    payload::{Binary, PlainText},
    ApiResponse,
};

/// A static file response.
#[cfg_attr(docsrs, doc(cfg(feature = "static-files")))]
#[derive(ApiResponse)]
#[oai(internal)]
pub enum StaticFileResponse {
    /// Ok
    #[oai(status = 200)]
    Ok(
        Binary<Body>,
        /// The ETag (or entity tag) HTTP response header is an identifier for a
        /// specific version of a resource. It lets caches be more efficient and
        /// save bandwidth, as a web server does not need to resend a full
        /// response if the content was not changed. Additionally, etags help to
        /// prevent simultaneous updates of a resource from overwriting each
        /// other ("mid-air collisions").
        ///
        /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag>
        #[oai(header = "etag")]
        Option<String>,
        /// The Last-Modified response HTTP header contains a date and time when
        /// the origin server believes the resource was last modified. It is
        /// used as a validator to determine if the resource is the same as the
        /// previously stored one. Less accurate than an ETag header, it is a
        /// fallback mechanism. Conditional requests containing
        /// If-Modified-Since or If-Unmodified-Since headers make use of this
        /// field.
        ///
        /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified>
        #[oai(header = "last-modified")]
        Option<String>,
    ),
    /// Not modified
    ///
    /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/304>
    #[oai(status = 304)]
    NotModified,
    /// Bad request
    ///
    /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/400>
    #[oai(status = 400)]
    BadRequest,
    /// Resource was not found
    #[oai(status = 404)]
    NotFound,
    /// Precondition failed
    ///
    /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/412>
    #[oai(status = 412)]
    PreconditionFailed,
    /// Range not satisfiable
    ///
    /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/416>
    #[oai(status = 416)]
    RangeNotSatisfiable(
        /// The Content-Range response HTTP header indicates where in a full
        /// body message a partial message belongs.
        ///
        /// Reference: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range>
        #[oai(header = "content-range")]
        String,
    ),
    /// Internal server error
    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

impl StaticFileResponse {
    /// Create a static file response.
    pub fn new(res: Result<poem::web::StaticFileResponse, StaticFileError>) -> Self {
        res.into()
    }
}

impl From<Result<poem::web::StaticFileResponse, StaticFileError>> for StaticFileResponse {
    fn from(res: Result<poem::web::StaticFileResponse, StaticFileError>) -> Self {
        match res {
            Ok(poem::web::StaticFileResponse::Ok {
                body,
                etag,
                last_modified,
                ..
            }) => StaticFileResponse::Ok(Binary(body), etag, last_modified),
            Ok(poem::web::StaticFileResponse::NotModified) => StaticFileResponse::NotModified,
            Err(
                StaticFileError::MethodNotAllowed(_)
                | StaticFileError::NotFound(_)
                | StaticFileError::InvalidPath
                | StaticFileError::Forbidden(_),
            ) => StaticFileResponse::NotFound,
            Err(StaticFileError::PreconditionFailed) => StaticFileResponse::PreconditionFailed,
            Err(StaticFileError::RangeNotSatisfiable { size }) => {
                StaticFileResponse::RangeNotSatisfiable(format!("*/{}", size))
            }
            Err(StaticFileError::Io(_)) => StaticFileResponse::BadRequest,
        }
    }
}
