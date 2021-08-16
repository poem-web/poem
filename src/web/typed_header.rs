use typed_headers::{Header, HeaderMapExt};

use crate::{
    body::Body,
    error::{Error, Result},
    request::Request,
    web::{FromRequest, RequestParts},
};

/// An extractor that extracts a typed header value.
pub struct TypedHeader<T>(pub T);

#[async_trait::async_trait]
impl<'a, T: Header> FromRequest<'a> for TypedHeader<T> {
    async fn from_request(parts: &'a RequestParts, body: &mut Option<Body>) -> Result<Self> {
        let value = parts.headers.typed_get::<T>().map_err(Error::bad_request)?;
        Ok(Self(value.ok_or_else(|| {
            Error::bad_request(anyhow::anyhow!("bad request"))
        })?))
    }
}
