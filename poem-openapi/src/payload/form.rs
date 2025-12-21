use std::ops::{Deref, DerefMut};

use poem::{FromRequest, Request, RequestBody, Result};
use serde::de::DeserializeOwned;

use crate::{
    error::ParseRequestPayloadError,
    payload::{ParsePayload, Payload},
    registry::{MetaSchemaRef, Registry},
    types::Type,
};

/// A URL-encoded form payload (`application/x-www-form-urlencoded`).
///
/// This type uses [`serde_html_form`](https://docs.rs/serde_html_form) to
/// parse form data, which properly handles HTML form encoding including
/// array fields with repeated keys.
///
/// # Array/Vec Fields
///
/// For fields that are arrays or `Vec<T>`, form data can use either a single
/// value or repeated keys:
///
/// ```text
/// // Single value (will be parsed as a Vec with one element):
/// ids=123
///
/// // Multiple values with repeated keys:
/// ids=123&ids=456
/// ```
///
/// # Nested Objects
///
/// Note that `serde_html_form` does not support nested objects or bracket
/// notation (like `ids[0]=123`). For complex form data with nested structures,
/// consider using [`Json`](crate::payload::Json) or
/// [`Multipart`](crate::payload::Multipart) instead.
///
/// # Example
///
/// ```rust
/// use poem_openapi::{payload::Form, Object, OpenApi};
///
/// #[derive(Debug, serde::Deserialize, Object)]
/// struct LoginForm {
///     username: String,
///     password: String,
/// }
///
/// struct Api;
///
/// #[OpenApi]
/// impl Api {
///     #[oai(path = "/login", method = "post")]
///     async fn login(&self, form: Form<LoginForm>) {
///         // Access form fields
///         let username = &form.username;
///         let password = &form.password;
///     }
/// }
/// ```
///
/// # Example with Arrays
///
/// ```rust
/// use poem_openapi::{payload::{Form, Json}, Object, OpenApi};
///
/// #[derive(Debug, serde::Deserialize, Object)]
/// struct BatchRequest {
///     ids: Vec<u32>,
/// }
///
/// struct Api;
///
/// #[OpenApi]
/// impl Api {
///     #[oai(path = "/batch", method = "post")]
///     async fn batch(&self, form: Form<BatchRequest>) -> Json<Vec<u32>> {
///         // form.ids will contain all values from repeated 'ids' keys
///         Json(form.ids.clone())
///     }
/// }
/// ```
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
                    .is_some_and(|v| v == "x-www-form-urlencoded")))
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

impl<T: DeserializeOwned> ParsePayload for Form<T> {
    const IS_REQUIRED: bool = true;

    async fn from_request(req: &Request, body: &mut RequestBody) -> Result<Self> {
        let data = Vec::<u8>::from_request(req, body).await?;
        Ok(Self(serde_html_form::from_bytes(&data).map_err(
            |err| ParseRequestPayloadError {
                reason: err.to_string(),
            },
        )?))
    }
}

impl_apirequest_for_payload!(Form<T>, T: DeserializeOwned + Type);
