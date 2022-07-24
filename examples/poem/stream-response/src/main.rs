use futures_util::StreamExt;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::Tracing,
    web::{stream::StreamResponse, Path},
    EndpointExt, IntoResponse, Route, Server,
};
use reqwest::StatusCode;

#[handler]
async fn dex(Path(id_or_name): Path<String>) -> (StatusCode, impl IntoResponse) {
    let endpoint = format!("https://pokeapi.co/api/v2/pokemon/{}", id_or_name);
    let res = reqwest::get(endpoint).await.unwrap();
    let status = res.status();
    let st = res.bytes_stream().map(|item| item.unwrap());
    (
        status,
        StreamResponse::new(st).with_content_type("application/json"),
    )
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/dex/:id_or_name", get(dex)).with(Tracing);
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .name("poke-proxy")
        .run(app)
        .await
}
