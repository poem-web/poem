use std::ops::{Deref, DerefMut};

use bcs::{from_bytes, to_bytes};
use poem::{
    http::{header, StatusCode},
    FromRequest, IntoResponse, Request, RequestBody, Response, Result,
};
use poem_openapi::{
    error::ParseRequestPayloadError,
    impl_apirequest_for_payload,
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::Type,
    ApiResponse,
};
use serde::{Deserialize, Serialize};

const CONTENT_TYPE_STR: &str = "application/x-bcs";

/// A BCS payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bcs<T>(pub T)
where
    T: Type;

impl<T: Type> Deref for Bcs<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Type> DerefMut for Bcs<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Type> Payload for Bcs<T> {
    const CONTENT_TYPE: &'static str = CONTENT_TYPE_STR;

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

impl<T: Type + for<'b> Deserialize<'b>> ParsePayload for Bcs<T> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        let data = Vec::<u8>::from_request(request, body).await?;
        let value: T = from_bytes(&data).map_err(|err| ParseRequestPayloadError {
            reason: err.to_string(),
        })?;
        Ok(Self(value))
    }
}

impl<T: Serialize + Send + Type> IntoResponse for Bcs<T> {
    fn into_response(self) -> Response {
        let data = match to_bytes(&self.0) {
            Ok(data) => data,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(err.to_string())
            }
        };
        Response::builder()
            .header(header::CONTENT_TYPE, Self::CONTENT_TYPE)
            .body(data)
    }
}

impl<T: Serialize + Type> ApiResponse for Bcs<T> {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                status_range: None,
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

impl_apirequest_for_payload!(Bcs<T>, T: Type + for<'b> Deserialize<'b>);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bcs_payload_preserves_wire_format_and_rejects_trailing_data() {
        let mut response = Bcs(42_u64).into_response();
        assert_eq!(response.headers()[header::CONTENT_TYPE], CONTENT_TYPE_STR);
        let bytes = response.take_body().into_vec().await.unwrap();
        assert_eq!(bytes, 42_u64.to_le_bytes());

        let (request, mut body) = Request::builder().body(bytes.clone()).split();
        let decoded = Bcs::<u64>::from_request(&request, &mut body).await.unwrap();
        assert_eq!(decoded.0, 42);

        let mut invalid = bytes;
        invalid.push(0);
        let (request, mut body) = Request::builder().body(invalid).split();
        assert!(Bcs::<u64>::from_request(&request, &mut body).await.is_err());
    }
}
