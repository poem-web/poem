use std::ops::{Deref, DerefMut};

use poem::{FromRequest, IntoResponse, Request, RequestBody, Response, Result};

use crate::{
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::Type,
    ApiResponse,
};

/// A UTF8 string payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlainText<T>(pub T);

impl<T> Deref for PlainText<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for PlainText<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Send> Payload for PlainText<T> {
    const CONTENT_TYPE: &'static str = "text/plain; charset=utf-8";

    fn check_content_type(content_type: &str) -> bool {
        matches!(content_type.parse::<mime::Mime>(), Ok(content_type) if content_type.type_() == "text"
                && (content_type.subtype() == "plain"
                || content_type
                    .suffix()
                    .map_or(false, |v| v == "plain")))
    }

    fn schema_ref() -> MetaSchemaRef {
        String::schema_ref()
    }
}

#[poem::async_trait]
impl ParsePayload for PlainText<String> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        Ok(Self(String::from_request(request, body).await?))
    }
}

impl<T: Into<String> + Send> IntoResponse for PlainText<T> {
    fn into_response(self) -> Response {
        self.0.into().into_response()
    }
}

impl<T: Into<String> + Send> ApiResponse for PlainText<T> {
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

impl_apirequest_for_payload!(PlainText<String>);
