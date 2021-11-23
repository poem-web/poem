use poem::{
    get, handler,
    listener::{Listener, TcpListener},
    IntoResponse, Route, Server,
};

#[handler]
fn hello() -> impl IntoResponse {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000")
        .combine(TcpListener::bind("127.0.0.1:3001"))
        .combine(TcpListener::bind("127.0.0.1:3002"));
    Server::new(listener).run(app).await
}
