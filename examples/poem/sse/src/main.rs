use std::time::Instant;

use futures_util::StreamExt;
use poem::{
    handler,
    listener::TcpListener,
    route,
    route::get,
    web::{
        sse::{Event, SSE},
        Html,
    },
    Server,
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
async fn main() {
    let app = route().at("/", get(index)).at("/event", get(event));

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
