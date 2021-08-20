use poem::{
    async_trait, get, handler, route, Endpoint, EndpointExt, Middleware, Request, Response, Server,
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
    async fn call(&self, req: Request) -> poem::Result<Response> {
        println!("request: {}", req.uri().path());
        let res = self.0.call(req).await;
        match &res {
            Ok(resp) => {
                println!("response: {}", resp.status())
            }
            Err(err) => println!("error: {}", err),
        }
        res
    }
}

#[handler]
fn index() -> String {
    format!("hello")
}

#[tokio::main]
async fn main() {
    let app = route().at("/", get(index)).with(Log);
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
