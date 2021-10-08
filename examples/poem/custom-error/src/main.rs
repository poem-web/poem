use poem::{
    get, handler, http::StatusCode, listener::TcpListener, web::Json, IntoResponse, Response,
    Route, Server,
};
use serde::Serialize;

#[derive(Serialize)]
struct CustomError {
    message: String,
}

impl IntoResponse for CustomError {
    fn into_response(self) -> Response {
        Json(self)
            .with_status(StatusCode::BAD_REQUEST)
            .into_response()
    }
}

#[handler]
fn hello() -> Result<String, CustomError> {
    Err(CustomError {
        message: "custom error".to_string(),
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
