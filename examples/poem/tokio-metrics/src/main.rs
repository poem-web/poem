use std::time::Duration;

use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{TokioMetrics, Tracing},
    EndpointExt, Route, Server,
};

#[handler]
async fn a() -> &'static str {
    "a"
}

#[handler]
async fn b() -> &'static str {
    tokio::time::sleep(Duration::from_millis(10)).await;
    "b"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let metrics_a = TokioMetrics::new();
    let metrics_b = TokioMetrics::new();

    let app = Route::new()
        .at("/metrics/a", metrics_a.exporter())
        .at("/metrics/b", metrics_b.exporter())
        .at("/a", get(a).with(metrics_a))
        .at("/b", get(b).with(metrics_b))
        .with(Tracing);
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
