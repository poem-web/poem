use poem::{
    async_trait, get, handler, listener::TcpListener, Endpoint, EndpointExt, IntoResponse,
    Middleware, Request, Response, Route, Server,
};

struct Log;

impl<E: Endpoint> Middleware<E> for Log {
    type Output = LogImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        LogImpl(ep)
    }
}

struct LogImpl<E>(E);

#[async_trait]
impl<E: Endpoint> Endpoint for LogImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Self::Output {
        println!("request: {}", req.uri().path());
        let resp = self.0.call(req).await.into_response();
        if resp.status().is_success() {
            println!("response: {}", resp.status());
        } else {
            println!("error: {}", resp.status());
        }
        resp
    }
}

#[handler]
fn index() -> String {
    "hello".to_string()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).with(Log);
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
