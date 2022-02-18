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

/// An event stream payload.
///
/// Reference: <https://github.com/OAI/OpenAPI-Specification/issues/396#issuecomment-894718960>
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventStream<T> {
    stream: T,
    keep_alive: Option<Duration>,
}

impl<T> EventStream<T> {
    /// Create an event stream payload.
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

impl<T: Stream<Item = E> + Send + 'static, E: Type + ToJSON> IntoResponse for EventStream<T> {
    fn into_response(self) -> Response {
        let mut sse = SSE::new(
            self.stream
                .map(|value| serde_json::to_string(&value.to_json()))
                .take_while(|value| futures_util::future::ready(value.is_ok()))
                .map(|value| Event::message(value.unwrap())),
        );

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
