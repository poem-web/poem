use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use tokio::time::Duration;

use super::Event;
use crate::{body::Body, error::Result, response::Response, web::IntoResponse};

/// An SSE response.
pub struct SSE<T> {
    stream: T,
    keep_alive: Option<Duration>,
}

impl<T> SSE<T>
where
    T: Stream<Item = Event> + Send + 'static,
{
    /// Create an SSE response using an event stream.
    pub fn new(stream: T) -> Self {
        Self {
            stream,
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

impl<T> IntoResponse for SSE<T>
where
    T: Stream<Item = Event> + Send + 'static,
{
    fn into_response(self) -> Result<Response> {
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
