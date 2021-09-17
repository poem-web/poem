use poem::{FromRequest, IntoResponse, Request, RequestBody, Response};

use crate::{
    payload::{ParsePayload, Payload},
    registry::MetaSchemaRef,
    types::Type,
    ParseRequestError,
};

/// A UTF8 string payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlainText<T>(pub T);

impl<T> Payload for PlainText<T> {
    const CONTENT_TYPE: &'static str = "text/plain";

    fn schema_ref() -> MetaSchemaRef {
        String::schema_ref()
    }
}

#[poem::async_trait]
impl ParsePayload for PlainText<String> {
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(String::from_request(request, body).await.map_err(
            |err| ParseRequestError::ParseRequestBody {
                reason: err.to_string(),
            },
        )?))
    }
}

impl<T: Into<String> + Send> IntoResponse for PlainText<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type(Self::CONTENT_TYPE)
            .body(self.0.into())
    }
}
