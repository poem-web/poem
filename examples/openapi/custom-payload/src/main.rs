use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{Object, OpenApi, OpenApiService};
use serde::{Deserialize, Serialize};

use crate::bcs_payload::Bcs;

mod bcs_payload;

#[derive(Debug, Deserialize, Object, Serialize)]
struct MyStruct {
    first_name: String,
    last_name: String,
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/echo", method = "post")]
    async fn index(&self, input: Bcs<MyStruct>) -> Bcs<MyStruct> {
        input
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }

    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
