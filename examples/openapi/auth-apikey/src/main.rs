use hmac::{Hmac, NewMac};
use jwt::{SignWithKey, VerifyWithKey};
use poem::{
    error::InternalServerError, listener::TcpListener, web::Data, EndpointExt, Request, Result,
    Route,
};
use poem_openapi::{
    auth::ApiKey,
    payload::{Json, PlainText},
    Object, OpenApi, OpenApiService, SecurityScheme,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const SERVER_KEY: &[u8] = b"123456";

type ServerKey = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
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

async fn api_checker(req: &Request, api_key: ApiKey) -> Option<User> {
    let server_key = req.data::<ServerKey>().unwrap();
    VerifyWithKey::<User>::verify_with_key(api_key.key.as_str(), server_key).ok()
}

#[derive(Object)]
struct LoginRequest {
    username: String,
}

struct Api;

#[OpenApi]
#[allow(unused_variables)]
impl Api {
    /// This is just a demo, so you can log in with any username and password.
    #[oai(path = "/login", method = "post")]
    async fn login(
        &self,
        server_key: Data<&ServerKey>,
        req: Json<LoginRequest>,
    ) -> Result<PlainText<String>> {
        let token = User {
            username: req.0.username,
        }
        .sign_with_key(server_key.0)
        .map_err(InternalServerError)?;
        Ok(PlainText(token))
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
    let server_key = Hmac::<Sha256>::new_from_slice(SERVER_KEY).expect("valid server key");
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .data(server_key);

    poem::Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
