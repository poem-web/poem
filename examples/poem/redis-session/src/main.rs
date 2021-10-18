//! Run with
//!
//! ```not_rust
//! cargo run --example cookie_session
//! ```
use poem::{
    get, handler,
    listener::TcpListener,
    session::{CookieConfig, RedisSession, Session},
    EndpointExt, Route, Server,
};
use redis::{aio::ConnectionManager, Client};

#[handler]
async fn count(session: &Session) -> String {
    let count = match session.get::<i32>("count") {
        Some(value) => {
            let count = value + 1;
            session.set("count", count);
            count
        }
        None => {
            session.set("count", 1);
            1
        }
    };

    format!("Hello!\nHow many times have seen you: {}", count)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let client = Client::open("redis://127.0.0.1/").unwrap();

    let app = Route::new().at("/", get(count)).with(RedisSession::new(
        CookieConfig::default(),
        ConnectionManager::new(client).await.unwrap(),
    ));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
