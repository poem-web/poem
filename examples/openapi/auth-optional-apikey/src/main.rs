use poem::{listener::TcpListener, Request, Result, Route};
use poem_openapi::{auth::ApiKey, payload::PlainText, OpenApi, OpenApiService, SecurityScheme};

#[derive(Clone)]
struct User {
    username: String,
}

/// Required session authentication using a cookie named `session`.
#[derive(SecurityScheme)]
#[oai(
    ty = "api_key",
    key_name = "session",
    key_in = "cookie",
    checker = "session_checker"
)]
struct SessionAuthorization(User);

async fn session_checker(_req: &Request, api_key: ApiKey) -> Option<User> {
    match api_key.key.as_str() {
        "demo-token" => Some(User {
            username: "demo".to_string(),
        }),
        _ => None,
    }
}

/// Optional authentication: either a valid session cookie or anonymous access.
///
/// This pattern is useful for endpoints that personalize responses when a user
/// is logged in, but still allow anonymous requests.
#[derive(SecurityScheme)]
enum OptionalSessionAuthorization {
    Session(SessionAuthorization),
    #[oai(fallback)]
    Anonymous,
}

impl OptionalSessionAuthorization {
    fn username(&self) -> Option<&str> {
        match self {
            Self::Session(auth) => Some(auth.0.username.as_str()),
            Self::Anonymous => None,
        }
    }
}

struct Api;

#[OpenApi]
impl Api {
    /// Returns a personalized greeting when the `session` cookie is present and
    /// valid.
    ///
    /// Try authorizing with the cookie value `demo-token`.
    #[oai(path = "/hello", method = "get")]
    async fn hello(&self, auth: OptionalSessionAuthorization) -> PlainText<String> {
        match auth.username() {
            Some(username) => PlainText(format!("hello, {username}")),
            None => PlainText("hello, anonymous".to_string()),
        }
    }

    /// Returns `401 Unauthorized` unless the `session` cookie is valid.
    #[oai(path = "/protected", method = "get")]
    async fn protected(&self, auth: SessionAuthorization) -> PlainText<String> {
        PlainText(format!("protected hello, {}", auth.0.username))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service = OpenApiService::new(Api, "Optional Authentication Demo", "1.0")
        .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
