use poem::{listener::TcpListener, route, web::Data, EndpointExt};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, #[oai(extract)] data: Data<&i32>) -> PlainText<String> {
        PlainText(format!("{}", data.0))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:3000");
    let api_service = OpenApiService::new(Api)
        .title("Poem Extractor")
        .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui("http://localhost:3000");

    poem::Server::new(listener)
        .await?
        .run(route().nest("/api", api_service.data(100i32)).nest("/", ui))
        .await
}
