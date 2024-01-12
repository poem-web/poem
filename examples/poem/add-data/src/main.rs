use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use poem::{
    get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Path},
    EndpointExt, Route, Server,
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
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

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
