use poem::{
    get, handler, http::StatusCode, listener::TcpListener, web::Path, EndpointExt, Response, Route,
    Server,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/hello/:name", get(hello))
        .after(|resp| async move {
            if resp.status() == StatusCode::NOT_FOUND {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")
            } else {
                resp
            }
        });

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
