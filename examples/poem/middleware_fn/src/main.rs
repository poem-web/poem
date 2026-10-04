use poem::{
    Endpoint, EndpointExt, IntoResponse, Request, Response, Result, Route, Server, get, handler,
    listener::TcpListener,
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new().at("/", get(index)).around(log);
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
