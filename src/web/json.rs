use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

use crate::{Error, FromRequest, HeaderName, IntoResponse, Request, Response, Result};
use serde::Serialize;

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
        let data = serde_json::to_vec(&self.0).map_err(Error::internal_server_error)?;
        Response::builder()
            .header(HeaderName::CONTENT_TYPE, "application/json")
            .body(data.into())
    }
}
