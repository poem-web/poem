#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    use poem::{get, handler, http::Uri, listener::UnixListener, IntoResponse, Route, Server};

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
