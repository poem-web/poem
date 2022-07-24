use bytes::Bytes;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use poem::{web::stream::StreamResponse, IntoResponse, Response};

use crate::{
    payload::Payload,
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::{ToJSON, Type},
    ApiResponse,
};

/// A Json payload from a `Stream` of  `Bytes`.
pub struct JsonProxyStream<P> {
    stream: BoxStream<'static, Bytes>,
    _proxy: std::marker::PhantomData<P>,
}

impl<P> JsonProxyStream<P> {
    /// Create a Json payload from a Bytes Stream using a proxy type for schema.
    pub fn new(stream: impl Stream<Item = Bytes> + Send + 'static) -> Self {
        Self {
            stream: stream.boxed(),
            _proxy: std::marker::PhantomData::default(),
        }
    }
}

impl<P: Type + ToJSON> Payload for JsonProxyStream<P> {
    const CONTENT_TYPE: &'static str = "application/json";

    fn schema_ref() -> MetaSchemaRef {
        P::schema_ref()
    }

    fn register(registry: &mut Registry) {
        P::register(registry);
    }
}

impl<P: Type + ToJSON> IntoResponse for JsonProxyStream<P> {
    fn into_response(self) -> Response {
        StreamResponse::new(self.stream)
            .with_content_type(Self::CONTENT_TYPE)
            .into_response()
    }
}

impl<P: Type + ToJSON> ApiResponse for JsonProxyStream<P> {
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
        P::register(registry);
    }
}
