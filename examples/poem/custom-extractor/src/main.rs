use poem::{
    get, handler, http::StatusCode, listener::TcpListener, Error, FromRequest, Request,
    RequestBody, Result, Route, Server,
};

struct Token(String);

// Implements a token extractor
#[poem::async_trait]
impl<'a> FromRequest<'a> for Token {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let token = req
            .headers()
            .get("MyToken")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                Error::new_with_string("missing token").with_status(StatusCode::BAD_REQUEST)
            })?;
        Ok(Token(token.to_string()))
    }
}

#[handler]
async fn index(token: Token) {
    assert_eq!(token.0, "token123");
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
