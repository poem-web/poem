use typed_headers::{Header, HeaderMapExt};

use crate::error::{Error, Result};
use crate::request::Request;
use crate::web::FromRequest;

/// An extractor that extracts a typed header value.
pub struct TypedHeader<T>(pub T);

#[async_trait::async_trait]
impl<T: Header> FromRequest for TypedHeader<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let value = req.headers().typed_get::<T>().map_err(Error::bad_request)?;
        Ok(Self(value.ok_or_else(|| {
            Error::bad_request(anyhow::anyhow!("bad request"))
        })?))
    }
}
