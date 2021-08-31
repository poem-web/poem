use poem::{handler, middleware::Tracing, route, web::Path, EndpointExt, RouteMethod, Server};
use tracing_subscriber::{
    fmt, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(true)
                .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT),
        )
        .with(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .init();

    let app = route()
        .at("/hello/:name", RouteMethod::new().get(hello))
        .with(Tracing);

    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
