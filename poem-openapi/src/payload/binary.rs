use poem::{FromRequest, IntoResponse, Request, RequestBody, Response};

use crate::{
    payload::Payload,
    registry::{MetaSchema, MetaSchemaRef},
    ParseRequestError,
};

/// A binary payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Binary(pub Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for Binary {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[poem::async_trait]
impl Payload for Binary {
    const CONTENT_TYPE: &'static str = "application/octet-stream";

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(MetaSchema {
            format: Some("binary"),
            ..MetaSchema::new("string")
        })
    }

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(<Vec<u8>>::from_request(request, body).await.map_err(
            |err| ParseRequestError::ParseRequestBody {
                reason: err.to_string(),
            },
        )?))
    }
}

impl IntoResponse for Binary {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type(Self::CONTENT_TYPE)
            .body(self.0)
    }
}
