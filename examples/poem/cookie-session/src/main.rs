use poem::{
    get, handler,
    listener::TcpListener,
    session::{CookieConfig, CookieSession, Session},
    EndpointExt, Route, Server,
};

#[handler]
async fn count(session: &Session) -> String {
    let count = session.get::<i32>("count").unwrap_or(0) + 1;
    session.set("count", count);
    format!("Hello!\nHow many times have seen you: {count}")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", get(count))
        .with(CookieSession::new(CookieConfig::default().secure(false)));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
