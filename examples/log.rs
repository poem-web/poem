///feature logger must be enabled
use poem::{
    middleware::{log, Logger},
    prelude::*,
    web::Path,
};

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt()
    //     .with_max_level(Level::DEBUG)
    //     .init();

    log::start();

    let app = route()
        .at("/hello/:name", get(hello))
        .with(Logger::default());
    serve(app).run("127.0.0.1:8000").await.unwrap();
}
