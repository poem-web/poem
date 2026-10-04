use poem::{
    Endpoint, EndpointExt, Error, Middleware, Request, Result, Route, Server, get, handler,
    http::StatusCode,
    listener::TcpListener,
    web::{
        headers,
        headers::{HeaderMapExt, authorization::Basic},
    },
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

impl<E: Endpoint> Endpoint for BasicAuthEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Basic>>()
            && auth.0.username() == self.username
            && auth.0.password() == self.password
        {
            return self.ep.call(req).await;
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(index)).with(BasicAuth {
        username: "test".to_string(),
        password: "123456".to_string(),
    });
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
