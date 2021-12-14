use crate::{
    http::{header, StatusCode, Uri},
    IntoResponse, Response,
};

/// A redirect response.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{header, HeaderValue, StatusCode, Uri},
///     web::Redirect,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// async fn index() -> Redirect {
///     Redirect::moved_permanent(Uri::from_static("https://www.google.com"))
/// }
///
/// let app = Route::new().at("/", get(index));
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app.call(Request::default()).await.unwrap();
/// assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
/// assert_eq!(
///     resp.headers().get(header::LOCATION),
///     Some(&HeaderValue::from_static("https://www.google.com/"))
/// );
/// # });
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_redirect {
        ($fn:ident, $status:ident) => {
            #[test]
            fn $fn() {
                let resp = Redirect::$fn(Uri::from_static("https://example.com/")).into_response();
                assert_eq!(resp.status(), StatusCode::$status);
                assert_eq!(
                    resp.headers()
                        .get(header::LOCATION)
                        .and_then(|value| value.to_str().ok()),
                    Some("https://example.com/")
                );
            }
        };
    }

    test_redirect!(permanent, PERMANENT_REDIRECT);
    test_redirect!(moved_permanent, MOVED_PERMANENTLY);
    test_redirect!(see_other, SEE_OTHER);
    test_redirect!(temporary, TEMPORARY_REDIRECT);
}
