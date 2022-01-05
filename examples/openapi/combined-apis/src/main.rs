use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{OpenApi, OpenApiService};

struct Api1;

#[OpenApi]
impl Api1 {
    #[oai(path = "/a", method = "get")]
    async fn test(&self) {}
}

struct Api2;

#[OpenApi]
impl Api2 {
    #[oai(path = "/b", method = "post")]
    async fn test1(&self) {}

    #[oai(path = "/b", method = "get")]
    async fn test2(&self) {}
}

struct Api3;

#[OpenApi]
impl Api3 {
    #[oai(path = "/c", method = "post")]
    async fn test1(&self) {}

    #[oai(path = "/c", method = "get")]
    async fn test2(&self) {}
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service = OpenApiService::new((Api1, Api2, Api3), "Combined APIs", "1.0")
        .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
