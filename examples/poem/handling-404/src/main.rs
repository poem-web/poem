use poem::{
    EndpointExt, Response, Route, Server, error::NotFoundError, get, handler, http::StatusCode,
    listener::TcpListener, web::Path,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {name}")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app =
        Route::new()
            .at("/hello/:name", get(hello))
            .catch_error(|_: NotFoundError| async move {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")
            });

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
