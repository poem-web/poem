use std::ops::{Deref, DerefMut};

use headers::{Header, HeaderMapExt};

use crate::{error::ParseTypedHeaderError, FromRequest, Request, RequestBody, Result};

/// An extractor that extracts a typed header value.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{header, StatusCode},
///     web::{headers::Host, TypedHeader},
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// fn index(TypedHeader(host): TypedHeader<Host>) -> String {
///     host.hostname().to_string()
/// }
///
/// let app = Route::new().at("/", get(index));
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app
///     .call(
///         Request::builder()
///             .header(header::HOST, "example.com")
///             .finish(),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "example.com");
/// # });
/// ```
pub struct TypedHeader<T>(pub T);

impl<T> Deref for TypedHeader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TypedHeader<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: Header> FromRequest<'a> for TypedHeader<T> {
    type Error = ParseTypedHeaderError;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        let value = req.headers().typed_try_get::<T>()?;
        Ok(Self(value.ok_or_else(|| {
            ParseTypedHeaderError::HeaderRequired(T::name().to_string())
        })?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, web::headers::ContentLength, Endpoint};

    #[tokio::test]
    async fn test_typed_header_extractor() {
        #[handler(internal)]
        async fn index(content_length: TypedHeader<ContentLength>) {
            assert_eq!(content_length.0 .0, 3);
        }

        index
            .call(Request::builder().header("content-length", 3).body("abc"))
            .await;
    }
}
