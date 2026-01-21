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
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

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
