use poem::{Error, FromRequest, IntoResponse, Request, RequestBody, Response};
use serde_json::Value;

use crate::{
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    ApiResponse, ParseRequestError,
};

/// A JSON payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Json<T>(pub T);

impl<T: Type> Payload for Json<T> {
    const CONTENT_TYPE: &'static str = "application/json";

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

#[poem::async_trait]
impl<T: ParseFromJSON> ParsePayload for Json<T> {
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        let value = poem::web::Json::<Value>::from_request(request, body)
            .await
            .map_err(|err| ParseRequestError::ParseRequestBody {
                reason: Into::<Error>::into(err)
                    .reason()
                    .unwrap_or_default()
                    .to_string(),
            })?;
        let value =
            T::parse_from_json(value.0).map_err(|err| ParseRequestError::ParseRequestBody {
                reason: err.into_message(),
            })?;
        Ok(Self(value))
    }
}

impl<T: ToJSON> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        poem::web::Json(self.0.to_json()).into_response()
    }
}

impl<T: ToJSON> ApiResponse for Json<T> {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: None,
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
        T::register(registry);
    }
}
