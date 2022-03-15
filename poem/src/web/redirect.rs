use std::fmt::Display;

use crate::{
    http::{header, StatusCode},
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
///     test::TestClient,
///     web::Redirect,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// async fn index() -> Redirect {
///     Redirect::moved_permanent("https://www.google.com")
/// }
///
/// let app = Route::new().at("/", get(index));
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = TestClient::new(app).get("/").send().await;
/// resp.assert_status(StatusCode::MOVED_PERMANENTLY);
/// resp.assert_header(header::LOCATION, "https://www.google.com");
/// # });
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Redirect {
    status: StatusCode,
    uri: String,
}

impl Redirect {
    /// A simple `308` permanent redirect to a different location.
    pub fn permanent(uri: impl Display) -> Self {
        Self {
            status: StatusCode::PERMANENT_REDIRECT,
            uri: uri.to_string(),
        }
    }

    /// A simple `301` permanent redirect to a different location.
    pub fn moved_permanent(uri: impl Display) -> Self {
        Self {
            status: StatusCode::MOVED_PERMANENTLY,
            uri: uri.to_string(),
        }
    }

    /// A simple `303` redirect to a different location.
    pub fn see_other(uri: impl Display) -> Self {
        Self {
            status: StatusCode::SEE_OTHER,
            uri: uri.to_string(),
        }
    }

    /// A simple `307` temporary redirect to a different location.
    pub fn temporary(uri: impl Display) -> Self {
        Self {
            status: StatusCode::TEMPORARY_REDIRECT,
            uri: uri.to_string(),
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        self.status
            .with_header(header::LOCATION, self.uri)
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
                let resp = Redirect::$fn("https://example.com/").into_response();
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
