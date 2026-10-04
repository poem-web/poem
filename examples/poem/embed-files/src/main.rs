use poem::{
    Route, Server,
    endpoint::{EmbeddedFileEndpoint, EmbeddedFilesEndpoint},
    listener::TcpListener,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "files"]
pub struct Files;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new()
        .at("/", EmbeddedFileEndpoint::<Files>::new("index.html"))
        .nest("/files", EmbeddedFilesEndpoint::<Files>::new());
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
