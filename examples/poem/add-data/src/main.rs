use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use poem::{
    EndpointExt, Route, Server, get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Path},
};

struct AppState {
    clients: Mutex<HashMap<String, String>>,
}

#[handler]
fn set_state(Path(name): Path<String>, state: Data<&Arc<AppState>>) -> String {
    let mut store = state.clients.lock().unwrap();
    store.insert(name.to_string(), "some state object".to_string());
    "store updated".to_string()
}

#[handler]
fn get_state(Path(name): Path<String>, state: Data<&Arc<AppState>>) -> String {
    let store = state.clients.lock().unwrap();
    let message = store.get(&name).unwrap();
    message.to_string()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let state = Arc::new(AppState {
        clients: Mutex::new(HashMap::new()),
    });

    let app = Route::new()
        .at("/hello/:name", get(set_state))
        .at("/:name", get(get_state))
        .with(AddData::new(state));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .name("add-data")
        .run(app)
        .await
}
