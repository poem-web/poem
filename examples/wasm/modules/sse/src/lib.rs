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
        poem::wasi::IntervalStream::new(Duration::from_secs(1))
            .map(move |_| Event::message(now.elapsed().as_secs().to_string())),
    )
    .keep_alive(Duration::from_secs(5))
}

#[no_mangle]
fn start() {
    poem::wasi::run(index);
}
