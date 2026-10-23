use poem::{
    Route, Server, get, handler,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
};
use tokio::time::Duration;

#[handler]
fn index() -> &'static str {
    "hello world"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(index));

    let listener = TcpListener::bind("0.0.0.0:3000").rustls(async_stream::stream! {
        loop {
            if let Ok(tls_config) = load_tls_config() {
                yield tls_config;
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
    Server::new(listener).run(app).await
}

fn load_tls_config() -> Result<RustlsConfig, std::io::Error> {
    Ok(RustlsConfig::new().fallback(
        RustlsCertificate::new()
            .cert(std::fs::read("examples/poem/tls-reload/src/cert.pem")?)
            .key(std::fs::read("examples/poem/tls-reload/src/key.pem")?),
    ))
}
