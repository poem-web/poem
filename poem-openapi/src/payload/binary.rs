use poem::{FromRequest, IntoResponse, Request, RequestBody, Response};

use crate::{
    payload::{ParsePayload, Payload},
    poem::Error,
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchema, MetaSchemaRef, Registry},
    ApiResponse, ParseRequestError,
};

/// A binary payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Binary<T>(pub T);

impl<T: Send> Payload for Binary<T> {
    const CONTENT_TYPE: &'static str = "application/octet-stream";

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            format: Some("binary"),
            ..MetaSchema::new("string")
        }))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<Vec<u8>> {
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(<Vec<u8>>::from_request(request, body).await.map_err(
            |err| {
                ParseRequestError::ParseRequestBody {
                    reason: Into::<Error>::into(err)
                        .reason()
                        .unwrap_or_default()
                        .to_string(),
                }
            },
        )?))
    }
}

impl<T: Into<Vec<u8>> + Send> IntoResponse for Binary<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type(Self::CONTENT_TYPE)
            .body(self.0.into())
    }
}

impl<T: Into<Vec<u8>> + Send> ApiResponse for Binary<T> {
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

    fn register(_registry: &mut Registry) {}
}
