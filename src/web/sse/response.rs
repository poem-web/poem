use bytes::Bytes;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use tokio::time::Duration;

use super::Event;
use crate::{Body, IntoResponse, Response};

/// An SSE response.
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
            stream = futures_util::stream::select(
                stream,
                tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(duration))
                    .map(move |_| Ok(comment.clone())),
            )
            .boxed();
        }

        Response::builder()
            .content_type("text/event-stream")
            .body(Body::from_async_read(tokio_util::io::StreamReader::new(
                stream,
            )))
    }
}
