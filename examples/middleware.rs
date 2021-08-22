use poem::{async_trait, get, route, Endpoint, EndpointExt, Middleware, Request, Response, Server};

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
    async fn call(&self, req: Request) -> Response {
        println!("request: {}", req.uri().path());
        let resp = self.0.call(req).await;
        if resp.status().is_success() {
            println!("response: {}", resp.status());
        } else {
            println!("error: {}", resp.status());
        }
        resp
    }
}

#[get]
fn index() -> String {
    format!("hello")
}

#[tokio::main]
async fn main() {
    let app = route().at("/", index).with(Log);
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
