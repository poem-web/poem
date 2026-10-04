use poem::{
    IntoResponse, Route, Server, get, handler,
    listener::{Listener, TcpListener},
};

#[handler]
fn hello() -> impl IntoResponse {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(hello));
    let listener = TcpListener::bind("0.0.0.0:3000")
        .combine(TcpListener::bind("0.0.0.0:3001"))
        .combine(TcpListener::bind("0.0.0.0:3002"));
    Server::new(listener).run(app).await
}
