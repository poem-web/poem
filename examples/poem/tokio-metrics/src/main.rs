use std::time::Duration;

use poem::{
    EndpointExt, Route, Server, get, handler,
    listener::TcpListener,
    middleware::{TokioMetrics, Tracing},
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let metrics_a = TokioMetrics::new();
    let metrics_b = TokioMetrics::new();

    let app = Route::new()
        .at("/metrics/a", metrics_a.exporter())
        .at("/metrics/b", metrics_b.exporter())
        .at("/a", get(a).with(metrics_a))
        .at("/b", get(b).with(metrics_b))
        .with(Tracing);
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
