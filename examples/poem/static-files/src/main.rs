use poem::{Route, Server, endpoint::StaticFilesEndpoint, listener::TcpListener};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().nest(
        "/",
        StaticFilesEndpoint::new("./poem/static-files/files").show_files_listing(),
    );
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
