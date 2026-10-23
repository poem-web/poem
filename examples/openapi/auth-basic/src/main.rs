use poem::{Error, Result, Route, http::StatusCode, listener::TcpListener};
use poem_openapi::{OpenApi, OpenApiService, SecurityScheme, auth::Basic, payload::PlainText};

/// Basic authorization
///
/// - User: `test`
/// - Password: `123456`
#[derive(SecurityScheme)]
#[oai(ty = "basic")]
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let api_service =
        OpenApiService::new(Api, "Authorization Demo", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
