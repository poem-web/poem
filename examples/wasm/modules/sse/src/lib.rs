use std::time::{Duration, Instant};

use poem::{
    handler,
    web::sse::{Event, SSE},
    IntoResponse,
};
use tokio_stream::StreamExt;

#[handler]
async fn index() -> impl IntoResponse {
    let now = Instant::now();
    SSE::new(
        tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(Duration::from_secs(1)))
            .map(move |_| Event::message(now.elapsed().as_secs().to_string())),
    )
    .keep_alive(Duration::from_secs(5))
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn start() {
    poem_wasm::run(index).await;
}
