use std::collections::HashMap;

use poem::{
    EndpointExt, Route, Server, get, handler,
    i18n::{I18NResources, Locale},
    listener::TcpListener,
    middleware::Tracing,
    web::Path,
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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

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
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .name("hello-world")
        .run(app)
        .await
}
