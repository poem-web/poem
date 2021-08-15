///logger feature must be enabled
use poem::{middleware::log, prelude::*, web::Path};

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    log::start();

    let logger = log::Logger::default();

    let app = route().at("/hello/:name", get(hello)).with(logger);
    serve(app).run("127.0.0.1:3000").await.unwrap();
}
