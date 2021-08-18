use typed_headers::{Header, HeaderMapExt};

use crate::{Error, FromRequest, Request, RequestBody, Result};

/// An extractor that extracts a typed header value.
pub struct TypedHeader<T>(pub T);

#[async_trait::async_trait]
impl<'a, T: Header> FromRequest<'a> for TypedHeader<T> {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let value = req.headers().typed_get::<T>().map_err(Error::bad_request)?;
        Ok(Self(value.ok_or_else(|| {
            Error::bad_request(anyhow::anyhow!("bad request"))
        })?))
    }
}
