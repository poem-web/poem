use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON},
    Object, OpenApi, OpenApiService,
};

#[derive(Object)]
struct MyObject<T: ParseFromJSON + ToJSON> {
    value: T,
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/i32", method = "post")]
    async fn i32(&self, value: Json<MyObject<i32>>) -> Json<MyObject<i32>> {
        value
    }

    #[oai(path = "/string", method = "post")]
    async fn string(&self, value: Json<MyObject<String>>) -> Json<MyObject<String>> {
        value
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
