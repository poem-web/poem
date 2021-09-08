use crate::{
    http::{header, StatusCode, Uri},
    IntoResponse, Response,
};

/// A redirect response.
///
/// # Example
///
/// ```
/// use poem::{handler, http::Uri, web::Redirect};
///
/// #[handler]
/// async fn index() -> Redirect {
///     Redirect::moved_permanent(Uri::from_static("https://www.google.com"))
/// }
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Redirect {
    status: StatusCode,
    uri: Uri,
}

impl Redirect {
    /// A simple `308` permanent redirect to a different location.
    pub fn permanent(uri: Uri) -> Self {
        Self {
            status: StatusCode::PERMANENT_REDIRECT,
            uri,
        }
    }

    /// A simple `301` permanent redirect to a different location.
    pub fn moved_permanent(uri: Uri) -> Self {
        Self {
            status: StatusCode::MOVED_PERMANENTLY,
            uri,
        }
    }

    /// A simple `303` redirect to a different location.
    pub fn see_other(uri: Uri) -> Self {
        Self {
            status: StatusCode::SEE_OTHER,
            uri,
        }
    }

    /// A simple `307` temporary redirect to a different location.
    pub fn temporary(uri: Uri) -> Self {
        Self {
            status: StatusCode::TEMPORARY_REDIRECT,
            uri,
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        self.status
            .with_header(header::LOCATION, self.uri.to_string())
            .into_response()
    }
}
