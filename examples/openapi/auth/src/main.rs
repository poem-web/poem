use poem::{handler, http::StatusCode, listener::TcpListener, route, Error, Result};
use poem_openapi::{
    auth::{ApiKey, Basic, Bearer},
    payload::PlainText,
    OpenApi, OpenApiService, SecurityScheme,
};

/// Basic authorization
///
/// - User: `test`
/// - Password: `123456`
#[derive(SecurityScheme)]
#[oai(type = "basic")]
struct MyBasicAuthorization(Basic);

/// ApiKey authorization
///
/// key: `123456`
#[derive(SecurityScheme)]
#[oai(type = "api_key", key_name = "X-API-Key", in = "header")]
struct MyApiKeyAuthorization(ApiKey);

/// Github authorization
///
/// - client_id: `409f09d61d182d0ae9a0`
/// - client_secret: `622cfd4c7168c43e09b0db1a18675dbcc5c0808b`
#[derive(SecurityScheme)]
#[oai(
    type = "oauth2",
    flows(authorization_code(
        authorization_url = "https://github.com/login/oauth/authorize",
        token_url = "http://localhost:3000/proxy",
        scope(name = "public_repo", desc = "access to public repositories."),
        scope(name = "read:user", desc = "access to read a user's profile data.")
    ))
)]
struct GithubAuthorization(Bearer);

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/basic", method = "get")]
    async fn auth_basic(
        &self,
        #[oai(auth)] auth: MyBasicAuthorization,
    ) -> Result<PlainText<String>> {
        if auth.0.username != "test" || auth.0.password != "123456" {
            return Err(Error::new(StatusCode::UNAUTHORIZED));
        }
        Ok(PlainText("hello".to_string()))
    }

    #[oai(path = "/api_key", method = "get")]
    async fn auth_api_key(
        &self,
        #[oai(auth)] auth: MyApiKeyAuthorization,
    ) -> Result<PlainText<String>> {
        if auth.0.key != "123456" {
            return Err(Error::new(StatusCode::UNAUTHORIZED));
        }
        Ok(PlainText("hello".to_string()))
    }

    #[oai(path = "/oauth2", method = "get")]
    async fn auth_oauth2(
        &self,
        #[oai(auth("public_repo", "read:user"))] auth: GithubAuthorization,
    ) -> Result<PlainText<String>> {
        let client = reqwest::Client::new();
        let text = client
            .get("https://api.github.com/repositories")
            .bearer_auth(auth.0.token)
            .header("accept", "application/vnd.github.v3+json")
            .header("user-agent","Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.82 Safari/537.36")
            .send()
            .await
            .map_err(Error::internal_server_error)?
            .text()
            .await
            .map_err(Error::internal_server_error)?;
        Ok(PlainText(text))
    }
}

// `https://github.com/login/oauth/access_token` does not support `Option` method, so make a proxy to avoid CORS issues.
#[handler]
async fn oauth_token_url_proxy(req: &poem::Request, body: poem::Body) -> Result<poem::Response> {
    let cli = reqwest::Client::new();
    let body = body.into_vec().await?;
    let resp = cli
        .request(
            req.method().clone(),
            format!(
                "https://github.com/login/oauth/access_token?{}",
                req.uri().query().unwrap_or_default()
            ),
        )
        .headers(req.headers().clone())
        .body(body)
        .send()
        .await
        .map_err(Error::bad_request)?;

    let mut r = poem::Response::default();
    r.set_status(resp.status());
    *r.headers_mut() = resp.headers().clone();
    r.set_body(resp.bytes().await.map_err(Error::bad_request)?);
    Ok(r)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:3000");
    let api_service = OpenApiService::new(Api)
        .title("Authorization Demo")
        .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui("http://localhost:3000");

    poem::Server::new(listener)
        .await?
        .run(
            route()
                .at("/proxy", oauth_token_url_proxy)
                .nest("/api", api_service)
                .nest("/", ui),
        )
        .await
}
