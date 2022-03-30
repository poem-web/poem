use std::ops::{Deref, DerefMut};

use poem::{
    error::ParseFormError,
    http::{header, HeaderValue},
    Request, RequestBody, Result,
};
use serde::de::DeserializeOwned;

use crate::{
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
        let content_type = req.headers().get(header::CONTENT_TYPE);
        if content_type
            != Some(&HeaderValue::from_static(
                "application/x-www-form-urlencoded",
            ))
        {
            return match content_type.and_then(|value| value.to_str().ok()) {
                Some(ty) => Err(ParseFormError::InvalidContentType(ty.to_string()).into()),
                None => Err(ParseFormError::ContentTypeRequired.into()),
            };
        }

        Ok(Self(
            serde_urlencoded::from_bytes(&body.take()?.into_vec().await?)
                .map_err(ParseFormError::UrlDecode)?,
        ))
    }
}

impl_apirequest_for_payload!(Form<T>, T: DeserializeOwned + Type);
