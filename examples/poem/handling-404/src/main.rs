use poem::{
    error::NotFoundError, get, handler, http::StatusCode, listener::TcpListener, web::Path,
    EndpointExt, Response, Route, Server,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {name}")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app =
        Route::new()
            .at("/hello/:name", get(hello))
            .catch_error(|_: NotFoundError| async move {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")
            });

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
