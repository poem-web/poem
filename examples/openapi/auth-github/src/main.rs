use poem::{
    error::{BadRequest, InternalServerError},
    handler,
    listener::TcpListener,
    Result, Route, Server,
};
use poem_openapi::{
    auth::Bearer, payload::PlainText, OAuthScopes, OpenApi, OpenApiService, SecurityScheme,
};

#[derive(OAuthScopes)]
enum GithubScopes {
    #[oai(rename = "public_repo")]
    PublicRepo,
}

/// Github authorization
///
/// - client_id: `409f09d61d182d0ae9a0`
/// - client_secret: `622cfd4c7168c43e09b0db1a18675dbcc5c0808b`
#[derive(SecurityScheme)]
#[oai(
    ty = "oauth2",
    flows(authorization_code(
        authorization_url = "https://github.com/login/oauth/authorize",
        token_url = "http://localhost:3000/proxy",
        scopes = "GithubScopes"
    ))
)]
struct GithubAuthorization(Bearer);

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/repositories", method = "get")]
    async fn repositories(
        &self,
        #[oai(scope = "GithubScopes::PublicRepo")] auth: GithubAuthorization,
    ) -> Result<PlainText<String>> {
        let client = reqwest::Client::new();
        let text = client
            .get("https://api.github.com/repositories")
            .bearer_auth(auth.0.token)
            .header("accept", "application/vnd.github.v3+json")
            .header("user-agent","Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.82 Safari/537.36")
            .send()
            .await
            .map_err(InternalServerError)?
            .text()
            .await
            .map_err(InternalServerError)?;
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
        .map_err(BadRequest)?;

    let mut r = poem::Response::default();
    r.set_status(resp.status());
    *r.headers_mut() = resp.headers().clone();
    r.set_body(resp.bytes().await.map_err(BadRequest)?);
    Ok(r)
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

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(
            Route::new()
                .at("/proxy", oauth_token_url_proxy)
                .nest("/api", api_service)
                .nest("/", ui),
        )
        .await
}
