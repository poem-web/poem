use std::time::{Duration, Instant};

use poem::{
    handler,
    runtime::wasi::IntervalStream,
    web::sse::{Event, SSE},
    IntoResponse,
};
use tokio_stream::StreamExt;

#[handler]
async fn index() -> impl IntoResponse {
    let now = Instant::now();
    SSE::new(
        IntervalStream::new(Duration::from_secs(1))
            .map(move |_| Event::message(now.elapsed().as_secs().to_string())),
    )
    .keep_alive(Duration::from_secs(5))
}

#[no_mangle]
fn start() {
    poem::runtime::wasi::run(index);
}
