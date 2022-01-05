use poem::{
    get, handler,
    http::StatusCode,
    listener::TcpListener,
    web::{
        headers,
        headers::{authorization::Basic, HeaderMapExt},
    },
    Endpoint, EndpointExt, Error, Middleware, Request, Result, Route, Server,
};

struct BasicAuth {
    username: String,
    password: String,
}

impl<E: Endpoint> Middleware<E> for BasicAuth {
    type Output = BasicAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        BasicAuthEndpoint {
            ep,
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}

struct BasicAuthEndpoint<E> {
    ep: E,
    username: String,
    password: String,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for BasicAuthEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Basic>>() {
            if auth.0.username() == self.username && auth.0.password() == self.password {
                return self.ep.call(req).await;
            }
        }
        Err(Error::from_status(StatusCode::UNAUTHORIZED))
    }
}

#[handler]
fn index() -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).with(BasicAuth {
        username: "test".to_string(),
        password: "123456".to_string(),
    });
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
