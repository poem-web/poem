use std::time::Instant;

use futures_util::StreamExt;
use poem::{
    get, handler,
    listener::TcpListener,
    web::{
        sse::{Event, SSE},
        Html,
    },
    Route, Server,
};
use tokio::time::Duration;

#[handler]
fn index() -> Html<&'static str> {
    Html(
        r#"
    <script>
    var eventSource = new EventSource('event');
    eventSource.onmessage = function(event) {
        document.write("<div>" + event.data + "</div>");
    }
    </script>
    "#,
    )
}

#[handler]
fn event() -> SSE {
    let now = Instant::now();
    SSE::new(
        tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(Duration::from_secs(1)))
            .map(move |_| Event::message(now.elapsed().as_secs().to_string())),
    )
    .keep_alive(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).at("/event", get(event));

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
            },
            Some(Duration::from_secs(5)),
        )
        .await
}
