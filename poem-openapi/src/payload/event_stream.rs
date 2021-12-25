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
pub struct EventStream<T>(T);

impl<T> EventStream<T> {
    /// Create an event stream payload.
    pub fn new(stream: T) -> Self {
        Self(stream)
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
}

impl<T: Stream<Item = E> + Send + 'static, E: Type + ToJSON> IntoResponse for EventStream<T> {
    fn into_response(self) -> Response {
        SSE::new(
            self.0
                .map(|value| serde_json::to_string(&value.to_json()))
                .take_while(|value| futures_util::future::ready(value.is_ok()))
                .map(|value| Event::message(value.unwrap())),
        )
        .into_response()
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
