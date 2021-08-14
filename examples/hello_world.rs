use poem::web::Path;
use poem::{get, route};

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", route::get(hello));

    poem::Server::new(app)
        .serve(&"127.0.0.1:3000".parse().unwrap())
        .await
        .unwrap();
}
