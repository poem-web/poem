use poem::{http::StatusCode, listener::TcpListener, Error, Request, Result, Route};
use poem_openapi::{
    auth::ApiKey, param::Query, payload::PlainText, OpenApi, OpenApiService, SecurityScheme,
};

struct User {
    username: String,
}

/// ApiKey authorization
#[derive(SecurityScheme)]
#[oai(
    type = "api_key",
    key_name = "X-API-Key",
    in = "header",
    checker = "api_checker"
)]
struct MyApiKeyAuthorization(User);

async fn api_checker(_: &Request, api_key: ApiKey) -> Option<User> {
    api_key.key.strip_prefix("key:").map(|username| User {
        username: username.to_string(),
    })
}

struct Api;

#[OpenApi]
#[allow(unused_variables)]
impl Api {
    /// This is just a demo, so you can log in with any username and password.
    #[oai(path = "/login", method = "get")]
    async fn login(&self, user: Query<String>, password: Query<String>) -> PlainText<String> {
        PlainText(format!("key:{}", user.0))
    }

    /// This API returns the currently logged in user.
    #[oai(path = "/hello", method = "get")]
    async fn hello(&self, auth: MyApiKeyAuthorization) -> PlainText<String> {
        PlainText(auth.0.username)
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
