use poem::{
    Result, Route, Server, error::ResponseError, get, handler, http::StatusCode,
    listener::TcpListener,
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(hello));
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
