use std::time::Duration;

use futures_util::{Stream, StreamExt};
use poem::{
    web::sse::{Event, SSE},
    IntoResponse, Response,
};

use crate::{
    payload::Payload,
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchema, MetaSchemaRef, Registry},
    types::{ToJSON, Type},
    ApiResponse,
};

type ToEventFn<T> = Box<dyn (FnMut(T) -> Event) + Send + 'static>;

/// An event stream payload.
///
/// Reference: <https://github.com/OAI/OpenAPI-Specification/issues/396#issuecomment-894718960>
pub struct EventStream<T: Stream + Send + 'static> {
    stream: T,
    keep_alive: Option<Duration>,
    to_event: Option<ToEventFn<T::Item>>,
}

impl<T: Stream + Send + 'static> EventStream<T> {
    /// Create an event stream payload.
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            keep_alive: None,
            to_event: None,
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

    /// Set a function used to convert the message to SSE event.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use poem::web::sse::Event;
    /// use poem_openapi::{payload::EventStream, types::ToJSON, Object};
    ///
    /// #[derive(Debug, Object)]
    /// struct MyEvent {
    ///     value: i32,
    /// }
    ///
    /// EventStream::new(futures_util::stream::iter(vec![
    ///     MyEvent { value: 1 },
    ///     MyEvent { value: 2 },
    ///     MyEvent { value: 3 },
    /// ]))
    /// .to_event(|event| {
    ///     let json = event.to_json_string();
    ///     Event::message(json).event_type("push")
    /// });
    /// ```
    #[must_use]
    pub fn to_event(self, f: impl FnMut(T::Item) -> Event + Send + 'static) -> Self {
        Self {
            to_event: Some(Box::new(f)),
            ..self
        }
    }
}

impl<T: Stream<Item = E> + Send + 'static, E: Type + ToJSON> Payload for EventStream<T> {
    const CONTENT_TYPE: &'static str = "text/event-stream";

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            items: Some(Box::new(E::schema_ref())),
            ..MetaSchema::new_with_format("array", "event-stream")
        }))
    }

    fn register(registry: &mut Registry) {
        E::register(registry);
    }
}

impl<T: Stream<Item = E> + Send + 'static, E: Type + ToJSON + 'static> IntoResponse
    for EventStream<T>
{
    fn into_response(self) -> Response {
        let mut sse = match self.to_event {
            Some(to_event) => SSE::new(self.stream.map(to_event)),
            None => SSE::new(
                self.stream
                    .map(|message| message.to_json_string())
                    .map(Event::message),
            ),
        };

        if let Some(keep_alive) = self.keep_alive {
            sse = sse.keep_alive(keep_alive);
        }

        sse.into_response()
    }
}

impl<T: Stream<Item = E> + Send + 'static, E: Type + ToJSON> ApiResponse for EventStream<T> {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: Self::CONTENT_TYPE,
                    schema: Self::schema_ref(),
                }],
                headers: vec![],
            }],
        }
    }

    fn register(registry: &mut Registry) {
        E::register(registry);
    }
}
