use std::task::Poll;

use bytes::Bytes;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use tokio::time::Duration;

use super::Event;
use crate::{Body, IntoResponse, Response};

/// An SSE response.
///
/// # Example
///
/// ```
/// use futures_util::stream;
/// use poem::{
///     handler,
///     http::StatusCode,
///     test::TestClient,
///     web::sse::{Event, SSE},
///     Endpoint, Request,
/// };
///
/// #[handler]
/// fn index() -> SSE {
///     SSE::new(stream::iter(vec![
///         Event::message("a"),
///         Event::message("b"),
///         Event::message("c"),
///     ]))
/// }
///
/// let cli = TestClient::new(index);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("data: a\n\ndata: b\n\ndata: c\n\n").await;
/// # });
/// ```
pub struct SSE {
    stream: BoxStream<'static, Event>,
    keep_alive: Option<Duration>,
}

impl SSE {
    /// Create an SSE response using an event stream.
    pub fn new(stream: impl Stream<Item = Event> + Send + 'static) -> Self {
        Self {
            stream: stream.boxed(),
            keep_alive: None,
        }
    }

    /// Set the keep alive interval.
    #[must_use]
    pub fn keep_alive(self, duration: Duration) -> Self {
        Self {
            keep_alive: Some(duration),
            ..self
        }
    }
}

impl IntoResponse for SSE {
    fn into_response(self) -> Response {
        let mut stream = self
            .stream
            .map(|event| Ok::<_, std::io::Error>(Bytes::from(event.to_string())))
            .boxed();
        if let Some(duration) = self.keep_alive {
            let comment = Bytes::from_static(b":\n\n");
            let mut interval =
                tokio::time::interval_at(tokio::time::Instant::now() + duration, duration);
            stream = futures_util::stream::poll_fn(move |cx| {
                if let Poll::Ready(msg) = stream.poll_next_unpin(cx) {
                    return Poll::Ready(msg);
                }
                interval.poll_tick(cx).map(|_| Some(Ok(comment.clone())))
            })
            .boxed();
        }

        Response::builder()
            .content_type("text/event-stream")
            .header("X-Accel-Buffering", "no")
            .header("Cache-Control", "no-cache")
            .body(Body::from_async_read(tokio_util::io::StreamReader::new(
                stream,
            )))
    }
}
