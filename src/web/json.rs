use std::ops::{Deref, DerefMut};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{Error, Result},
    http::header,
    request::Request,
    response::Response,
    web::{FromRequest, IntoResponse},
};

/// JSON extractor and response.
///
/// # Extractor
///
/// To extract the specified type of JSON from the body, `T` must implement
/// [`serde::Deserialize`].
///
/// ```
/// use serde::Deserialize;
/// use poem::web::Json;
///
/// #[derive(Deserialize)]
/// struct User {
///     name: String,
/// }
///
/// async fn index(Json(user) : Json<User>) -> String {
///     format!("welcome {}!", user.name)
/// }
/// ```
///
/// # Response
///
/// To serialize the specified type to JSON, `T` must implement
/// [`serde::Serialize`].
///
/// ```
/// use serde::Serialize;
/// use poem::web::Json;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
/// }
///
/// async fn index() -> Json<User> {
///     Json(User { name: "sunli".to_string() })
/// }
/// ```
pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<T: DeserializeOwned> FromRequest for Json<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let data = req.take_body().into_bytes().await?;
        Ok(Self(
            serde_json::from_slice(&data).map_err(Error::bad_request)?,
        ))
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Result<Response> {
        let data = serde_json::to_vec(&self.0).map_err(Error::bad_request)?;
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(data.into())
    }
}
