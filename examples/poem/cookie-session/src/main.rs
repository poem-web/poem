//! Run with
//!
//! ```not_rust
//! cargo run --example cookie_session
//! ```
use async_session::{MemoryStore, Session, SessionStore};
use poem::{
    handler,
    listener::TcpListener,
    middleware::CookieJarManager,
    route,
    route::get,
    web::{Cookie, CookieJar, Data},
    Endpoint, EndpointExt, Request, Server,
};

#[handler]
async fn count(store: Data<&MemoryStore>, cookie_jar: &CookieJar) -> String {
    let count = match cookie_jar.get("sid") {
        Some(cookie) => {
            match store
                .load_session(cookie.value().to_string())
                .await
                .unwrap()
            {
                Some(mut session) => {
                    // load the count number from session and increment it
                    let count = session.get::<i32>("count").unwrap_or(1) + 1;
                    // save session with the new count
                    session.insert("count", count).unwrap();
                    session.set_cookie_value(cookie.value().to_string());
                    store.store_session(session).await.unwrap().unwrap();
                    count
                }
                None => {
                    init_session(&store, cookie_jar).await;
                    1
                }
            }
        }
        None => {
            init_session(&store, cookie_jar).await;
            1
        }
    };

    format!("Hello!\nHow many times have seen you: {}", count)
}

async fn init_session(store: &Data<&MemoryStore>, cookie_jar: &CookieJar) {
    let sid = store.store_session(Session::new()).await.unwrap().unwrap();
    // tell browser to save cookie which indicating the session
    cookie_jar.add(Cookie::new("sid", sid));
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    // `MemoryStore` just used as an example. Don't use this in production.
    let store = MemoryStore::new();

    let app = route()
        .at("/", get(count))
        .data(store)
        .with(CookieJarManager);
    app.call(Request::default()).await;
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
