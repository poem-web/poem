use std::ops::{Deref, DerefMut};

use poem::{FromRequest, IntoResponse, Request, RequestBody, Response, Result};
use serde_json::Value;

use crate::{
    error::ParseRequestPayloadError,
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::{ParseFromXML, ToXML, Type},
    ApiResponse,
};

/// A XML payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Xml<T>(pub T);

impl<T> Deref for Xml<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Xml<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Type> Payload for Xml<T> {
    const CONTENT_TYPE: &'static str = "application/xml; charset=utf-8";

    fn check_content_type(content_type: &str) -> bool {
        matches!(content_type.parse::<mime::Mime>(), Ok(content_type) if content_type.type_() == "application"
                && (content_type.subtype() == "xml"
                || content_type
                    .suffix()
                    .map_or(false, |v| v == "xml")))
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

#[poem::async_trait]
impl<T: ParseFromXML> ParsePayload for Xml<T> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        let data: Vec<u8> = FromRequest::from_request(request, body).await?;
        let value = if data.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&data).map_err(|err| ParseRequestPayloadError {
                reason: err.to_string(),
            })?
        };

        let value = T::parse_from_xml(Some(value)).map_err(|err| ParseRequestPayloadError {
            reason: err.into_message(),
        })?;
        Ok(Self(value))
    }
}

impl<T: ToXML> IntoResponse for Xml<T> {
    fn into_response(self) -> Response {
        poem::web::Xml(self.0.to_xml()).into_response()
    }
}

impl<T: ToXML> ApiResponse for Xml<T> {
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
        T::register(registry);
    }
}

impl_apirequest_for_payload!(Xml<T>, T: ParseFromXML);
