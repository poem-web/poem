use poem::{
    EndpointExt, Route, Server, get, handler, listener::TcpListener,
    middleware::TowerLayerCompatExt,
};
use tokio::time::Duration;
use tower::limit::RateLimitLayer;

#[handler]
fn hello() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at(
        "/",
        get(hello).with(RateLimitLayer::new(5, Duration::from_secs(30)).compat()),
    );
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
