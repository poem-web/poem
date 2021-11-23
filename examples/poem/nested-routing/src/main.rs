use poem::{get, handler, listener::TcpListener, Route, Server};

#[handler]
fn hello() -> String {
    "hello".to_string()
}

fn api() -> Route {
    Route::new().at("/hello", get(hello))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug")
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().nest("/api", api());
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
