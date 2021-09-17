use super::Html;
use crate::{http::StatusCode, IntoResponse, Response};

/// Template response using [`askama`](https://crates.io/crates/askama).
pub struct Template<T>(pub T);

impl<T: askama::Template + Send> IntoResponse for Template<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(s) => s.into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string()),
        }
    }
}

/// Template response with content-type "text/html" using [`askama`](https://crates.io/crates/askama).
pub struct HtmlTemplate<T>(pub T);

impl<T: askama::Template + Send> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(s) => Html(s).into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string()),
        }
    }
}
