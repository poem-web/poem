use poem::{
    handler, listener::TcpListener, middleware::TowerLayerCompatExt, route, route::get,
    EndpointExt, Server,
};
use tokio::time::Duration;
use tower::limit::RateLimitLayer;

#[handler]
fn hello() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() {
    let app = route().at(
        "/",
        get(hello).with(RateLimitLayer::new(5, Duration::from_secs(30)).compat()),
    );
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
