use poem::{
    EndpointExt, Route, Server, get, handler,
    listener::TcpListener,
    session::{CookieConfig, RedisStorage, ServerSession, Session},
};
use redis::{Client, aio::ConnectionManager};

#[handler]
async fn count(session: &Session) -> String {
    let count = session.get::<i32>("count").unwrap_or(0) + 1;
    session.set("count", count);
    format!("Hello!\nHow many times have seen you: {count}")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let client = Client::open("redis://127.0.0.1/").unwrap();

    let app = Route::new().at("/", get(count)).with(ServerSession::new(
        CookieConfig::default().secure(false),
        RedisStorage::new(ConnectionManager::new(client).await.unwrap()),
    ));
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
