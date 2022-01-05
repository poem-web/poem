use poem::{http::StatusCode, listener::TcpListener, Error, Result, Route};
use poem_openapi::{auth::Basic, payload::PlainText, OpenApi, OpenApiService, SecurityScheme};

/// Basic authorization
///
/// - User: `test`
/// - Password: `123456`
#[derive(SecurityScheme)]
#[oai(type = "basic")]
struct MyBasicAuthorization(Basic);

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/basic", method = "get")]
    async fn auth_basic(&self, auth: MyBasicAuthorization) -> Result<PlainText<String>> {
        if auth.0.username != "test" || auth.0.password != "123456" {
            return Err(Error::from_status(StatusCode::UNAUTHORIZED));
        }
        Ok(PlainText(format!("hello: {}", auth.0.username)))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service =
        OpenApiService::new(Api, "Authorization Demo", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    poem::Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
