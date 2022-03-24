use poem::{handler, IntoResponse};

#[handler]
async fn index() -> impl IntoResponse {
    "Hello World from WASM!".with_header("Server", "Poem")
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn start() {
    poem_wasm::run(index).await;
}
