#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    use poem::{IntoResponse, Route, Server, get, handler, http::Uri, listener::UnixListener};

    #[handler]
    fn hello(uri: &Uri) -> impl IntoResponse {
        uri.path().to_string()
    }

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(hello));
    let listener = UnixListener::bind("./unix-socket");
    Server::new(listener).run(app).await
}

#[cfg(not(unix))]
fn main() {
    println!("This example works only on Unix systems!");
}
