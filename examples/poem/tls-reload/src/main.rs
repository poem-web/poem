use poem::{
    get, handler,
    listener::{Listener, TcpListener, TlsConfig},
    Route, Server,
};
use tokio::time::Duration;

#[handler]
fn index() -> &'static str {
    "hello world"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index));

    let listener = TcpListener::bind("127.0.0.1:3000").tls(async_stream::stream! {
        loop {
            if let Ok(tls_config) = load_tls_config() {
                yield tls_config;
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
    Server::new(listener).await?.run(app).await
}

fn load_tls_config() -> Result<TlsConfig, std::io::Error> {
    Ok(TlsConfig::new()
        .cert(std::fs::read("examples/poem/tls-reload/src/cert.pem")?)
        .key(std::fs::read("examples/poem/tls-reload/src/key.pem")?))
}
