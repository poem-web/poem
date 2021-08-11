use poem::middlewares::StripPrefix;
use poem::route::{self, Route};
use poem::EndpointExt;

async fn hello() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() {
    let route = Route::new().at("/hello", route::get(hello));
    let api = Route::new().at("/api/*", route.with(StripPrefix::new("/api")));

    poem::Server::new(api)
        .serve(&"127.0.0.1:3000".parse().unwrap())
        .await
        .unwrap();
}
