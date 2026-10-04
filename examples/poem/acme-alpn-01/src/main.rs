use poem::{
    EndpointExt, Route, Server, get, handler,
    listener::{
        Listener, TcpListener,
        acme::{AutoCert, LETS_ENCRYPT_PRODUCTION},
    },
    middleware::Tracing,
    web::Path,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {name}")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let auto_cert = AutoCert::builder()
        .directory_url(LETS_ENCRYPT_PRODUCTION)
        .domain("poem.rs")
        .build()?;

    let app = Route::new().at("/hello/:name", get(hello)).with(Tracing);

    Server::new(TcpListener::bind("0.0.0.0:443").acme(auto_cert))
        .name("hello-world")
        .run(app)
        .await
}
