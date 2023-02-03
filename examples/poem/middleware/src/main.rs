use poem::{
    async_trait, get, handler, listener::TcpListener, Endpoint, EndpointExt, IntoResponse,
    Middleware, Request, Response, Result, Route, Server,
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

    async fn call(&self, req: Request) -> Result<Self::Output> {
        println!("request: {}", req.uri().path());
        let res = self.0.call(req).await;

        match res {
            Ok(resp) => {
                let resp = resp.into_response();
                println!("response: {}", resp.status());
                Ok(resp)
            }
            Err(err) => {
                println!("error: {err}");
                Err(err)
            }
        }
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
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
