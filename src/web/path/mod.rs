mod de;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::error::{ErrorInvalidPathParams, ErrorMissingRouteParams};
use crate::route_recognizer::Params;
use crate::{Error, FromRequest, Request, Result};

#[derive(Debug)]
pub struct Path<T>(pub T);

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Path<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<T> FromRequest for Path<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(req: &mut Request) -> Result<Self> {
        let params = req
            .extensions_mut()
            .get::<Params>()
            .ok_or_else(|| Error::internal_server_error(ErrorMissingRouteParams))?;
        T::deserialize(de::PathDeserializer::new(params))
            .map_err(|_| Error::internal_server_error(ErrorInvalidPathParams))
            .map(Path)
    }
}
