use crate::{http::StatusCode, IntoResponse, Response};

/// Template response using [`askama`](https://crates.io/crates/askama).
pub struct Template<T>(T);

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
