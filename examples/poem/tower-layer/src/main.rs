use poem::{
    get, handler, listener::TcpListener, middleware::TowerLayerCompatExt, EndpointExt, Route,
    Server,
};
use tokio::time::Duration;
use tower::limit::RateLimitLayer;

#[handler]
fn hello() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at(
        "/",
        get(hello).with(RateLimitLayer::new(5, Duration::from_secs(30)).compat()),
    );
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
