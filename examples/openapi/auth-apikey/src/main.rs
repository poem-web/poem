use poem::{listener::TcpListener, Request, Result, Route};
use poem_openapi::{auth::ApiKey, payload::PlainText, OpenApi, OpenApiService, SecurityScheme};

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
impl Api {
    /// This is just a demo, so you can log in with any username and password.
    #[oai(path = "/login", method = "get")]
    async fn login(
        &self,
        #[oai(name = "user", in = "query")] user: String,
        #[oai(name = "password", in = "query")] _password: String,
    ) -> PlainText<String> {
        PlainText(format!("key:{}", user))
    }

    /// This API returns the currently logged in user.
    #[oai(path = "/hello", method = "get")]
    async fn hello(&self, #[oai(auth)] auth: MyApiKeyAuthorization) -> PlainText<String> {
        PlainText(auth.0.username)
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:3000");
    let api_service =
        OpenApiService::new(Api, "Authorization Demo", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    poem::Server::new(listener)
        .await?
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
