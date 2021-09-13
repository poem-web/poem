use poem::{
    async_trait, handler, listener::TcpListener, route, route::get, Endpoint, EndpointExt,
    IntoResponse, Middleware, Request, Response, Server,
};

struct Log;

impl<E: Endpoint> Middleware<E> for Log {
    type Output = LogImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
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
    format!("hello")
}

#[tokio::main]
async fn main() {
    let app = route().at("/", get(index)).with(Log);
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
