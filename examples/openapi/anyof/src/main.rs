use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{payload::Json, AnyOf, Object, OpenApi, OpenApiService};

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(Object, Debug, PartialEq)]
struct B {
    v3: f32,
}

#[derive(AnyOf, Debug, PartialEq)]
#[oai(discriminator_name = "type")]
enum MyObj {
    A(A),
    B(B),
    // C(bool),
    // D(i32),
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

    let api_service = OpenApiService::new(Api, "Anyof", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(
            Route::new()
                .nest("/api", api_service)
                .nest("/", ui)
                .at("/spec", spec),
        )
        .await
}
