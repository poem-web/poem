use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{payload::Json, Object, OpenApi, OpenApiService, Union};

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(Object, Debug, PartialEq)]
struct B {
    v3: f32,
}

#[derive(Union, Debug, PartialEq)]
#[oai(discriminator_name = "type")]
enum MyObj {
    A(A),
    B(B),
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/put", method = "post")]
    async fn index(&self, obj: Json<MyObj>) -> Json<MyObj> {
        obj
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service = OpenApiService::new(Api, "Union", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(
            Route::new()
                .nest("/api", api_service)
                .nest("/", ui)
                .at("/spec", spec)
                .at("/spec_yaml", spec_yaml),
        )
        .await
}
