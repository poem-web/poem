use poem::{
    error::ResponseError, get, handler, http::StatusCode, listener::TcpListener, IntoResponse,
    Response, Result, Route, Server,
};

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct CustomError {
    message: String,
}

impl ResponseError for CustomError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn as_response(&self) -> Response {
        self.to_string().with_status(self.status()).into_response()
    }
}

#[handler]
fn hello() -> Result<String> {
    Err(CustomError {
        message: "custom error".to_string(),
    }
    .into())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(hello));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
