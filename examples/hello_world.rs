use poem::route::{self, Route};
use poem::web::Path;

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let route = Route::new().at("/hello/:name", route::get(hello));

    poem::Server::new(route)
        .serve(&"127.0.0.1:3000".parse().unwrap())
        .await
        .unwrap();
}
