#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    use poem::{IntoResponse, Route, Server, get, handler, http::Uri, listener::UnixListener};

    #[handler]
    fn hello(uri: &Uri) -> impl IntoResponse {
        uri.path().to_string()
    }

    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(hello));
    let listener = UnixListener::bind("./unix-socket");
    Server::new(listener).run(app).await
}

#[cfg(not(unix))]
fn main() {
    println!("This example works only on Unix systems!");
}
