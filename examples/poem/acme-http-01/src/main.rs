use poem::{
    EndpointExt, Route, RouteScheme, Server, get, handler,
    listener::{
        Listener, TcpListener,
        acme::{AutoCert, ChallengeType, LETS_ENCRYPT_PRODUCTION},
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
        .challenge_type(ChallengeType::Http01)
        .build()?;

    let app = RouteScheme::new()
        .https(Route::new().at("/hello/:name", get(hello)))
        .http(auto_cert.http_01_endpoint())
        .with(Tracing);

    Server::new(
        TcpListener::bind("0.0.0.0:443")
            .acme(auto_cert)
            .combine(TcpListener::bind("0.0.0.0:80")),
    )
    .name("hello-world")
    .run(app)
    .await
}
