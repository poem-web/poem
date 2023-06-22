use std::collections::HashMap;

use poem::{
    get, handler,
    i18n::{I18NResources, Locale},
    listener::TcpListener,
    middleware::Tracing,
    web::Path,
    EndpointExt, Route, Server,
};

#[handler]
fn index(locale: Locale) -> String {
    locale
        .text("hello-world")
        .unwrap_or_else(|_| "error".to_string())
}

#[handler]
fn welcome_tuple(locale: Locale, Path(name): Path<String>) -> String {
    locale
        .text_with_args("welcome", (("name", name),))
        .unwrap_or_else(|_| "error".to_string())
}

#[handler]
fn welcome_hashmap(locale: Locale, Path(name): Path<String>) -> String {
    let mut args = HashMap::new();
    args.insert("name", name);

    locale
        .text_with_args("welcome", args)
        .unwrap_or_else(|_| "error".to_string())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let resources = I18NResources::builder()
        .add_path("resources")
        .build()
        .unwrap();

    let app = Route::new()
        .at("/", get(index))
        .at("/welcome_tuple/:name", get(welcome_tuple))
        .at("/welcome_hashmap/:name", get(welcome_hashmap))
        .with(Tracing)
        .data(resources);
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .name("hello-world")
        .run(app)
        .await
}
