use std::time::Instant;

use futures_util::StreamExt;
use poem::{
    handler, route,
    web::{
        sse::{Event, SSE},
        Html,
    },
    RouteMethod, Server,
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
    let app = route()
        .at("/", RouteMethod::new().get(index))
        .at("/event", RouteMethod::new().get(event));

    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
