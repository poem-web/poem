use poem::{
    endpoint::{EmbeddedFileEndpoint, EmbeddedFilesEndpoint},
    listener::TcpListener,
    Route, Server,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "files"]
pub struct Files;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", EmbeddedFileEndpoint::<Files>::new("index.html"))
        .nest("/files", EmbeddedFilesEndpoint::<Files>::new());
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
