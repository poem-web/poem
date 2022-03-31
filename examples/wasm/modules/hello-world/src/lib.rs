use poem::{handler, IntoResponse};

#[handler]
async fn index() -> impl IntoResponse {
    "Hello World from WASM!".with_header("Server", "Poem")
}

#[no_mangle]
fn start() {
    poem::runtime::wasi::run(index);
}
