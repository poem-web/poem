use poem::{
    get, handler, http::StatusCode, listener::TcpListener, FromRequest, IntoResponse, Request,
    RequestBody, Response, Route, Server,
};

struct Token(String);

// Error type for Token extractor
#[derive(Debug)]
struct MissingToken;

/// custom-error can also be reused
impl IntoResponse for MissingToken {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("missing token")
    }
}

// Implements a token extractor
#[poem::async_trait]
impl<'a> FromRequest<'a> for Token {
    type Error = MissingToken;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        let token = req
            .headers()
            .get("MyToken")
            .and_then(|value| value.to_str().ok())
            .ok_or(MissingToken)?;
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
