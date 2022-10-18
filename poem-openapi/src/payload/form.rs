use std::ops::{Deref, DerefMut};

use poem::{FromRequest, Request, RequestBody, Result};
use serde::de::DeserializeOwned;

use crate::{
    error::ParseRequestPayloadError,
    payload::{ParsePayload, Payload},
    registry::{MetaSchemaRef, Registry},
    types::Type,
};

/// A url encoded form payload.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Form<T>(pub T);

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Type> Payload for Form<T> {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";

    fn check_content_type(content_type: &str) -> bool {
        matches!(content_type.parse::<mime::Mime>(), Ok(content_type) if content_type.type_() == "application"
                && (content_type.subtype() == "x-www-form-urlencoded"
                || content_type
                    .suffix()
                    .map_or(false, |v| v == "x-www-form-urlencoded")))
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
impl<T: DeserializeOwned> ParsePayload for Form<T> {
    const IS_REQUIRED: bool = true;

    async fn from_request(req: &Request, body: &mut RequestBody) -> Result<Self> {
        let data: Vec<u8> = FromRequest::from_request(req, body).await?;
        Ok(Self(serde_urlencoded::from_bytes(&data).map_err(
            |err| ParseRequestPayloadError {
                reason: err.to_string(),
            },
        )?))
    }
}

impl_apirequest_for_payload!(Form<T>, T: DeserializeOwned + Type);
