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
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
