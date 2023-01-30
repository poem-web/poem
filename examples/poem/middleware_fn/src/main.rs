use poem::{
    get, handler, listener::TcpListener, Endpoint, EndpointExt, IntoResponse, Request, Response,
    Result, Route, Server,
};

#[handler]
fn index() -> String {
    "hello".to_string()
}

async fn log<E: Endpoint>(next: E, req: Request) -> Result<Response> {
    println!("request: {}", req.uri().path());
    let res = next.call(req).await;

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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).around(log);
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
