use poem::prelude::*;
use poem::web::Path;

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let log = poem::middleware::Logger::default();

    let app = route().at("/hello/:name", get(hello)).with(log);

    serve(app).run("127.0.0.1:3000").await.unwrap();
}
