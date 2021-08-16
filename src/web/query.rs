use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{
    body::Body,
    error::{Error, Result},
    web::{FromRequest, RequestParts},
};

/// An extractor that can deserialize some type from query string.
pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Query<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Query<T> {
    async fn from_request(parts: &'a RequestParts, _body: &mut Option<Body>) -> Result<Self> {
        serde_urlencoded::from_str(parts.uri.query().unwrap_or_default())
            .map_err(Error::bad_request)
            .map(Self)
    }
}
