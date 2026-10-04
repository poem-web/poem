use poem::{
    Error, FromRequest, Request, RequestBody, Result, Route, Server, get, handler,
    http::StatusCode, listener::TcpListener,
};

struct Token(String);

// Implements a token extractor
impl<'a> FromRequest<'a> for Token {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let token = req
            .headers()
            .get("MyToken")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Error::from_string("missing token", StatusCode::BAD_REQUEST))?;
        Ok(Token(token.to_string()))
    }
}

#[handler]
async fn index(token: Token) {
    assert_eq!(token.0, "token123");
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(index));
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
