use poem::{listener::TcpListener, service::Files, Route, Server};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().nest(
        "/",
        Files::new("./examples/poem/static-files/files").show_files_listing(),
    );
    let server = Server::new(TcpListener::bind("127.0.0.1:3000")).await?;
    server.run(app).await
}
