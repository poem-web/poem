use std::ops::{Deref, DerefMut};

use headers::{Header, HeaderMapExt};

use crate::{error::ParseTypedHeaderError, FromRequest, Request, RequestBody, Result};

/// An extractor that extracts a typed header value.
///
/// # Errors
///
/// - [`ParseTypedHeaderError`]
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{header, StatusCode},
///     test::TestClient,
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
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .get("/")
///     .header(header::HOST, "example.com")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("example.com").await;
/// # });
/// ```
#[derive(Debug)]
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

impl<T: Header> TypedHeader<T> {
    async fn internal_from_request(req: &Request) -> Result<Self, ParseTypedHeaderError> {
        let value = req.headers().typed_try_get::<T>()?;
        Ok(Self(value.ok_or_else(|| {
            ParseTypedHeaderError::HeaderRequired(T::name().to_string())
        })?))
    }
}

impl<'a, T: Header> FromRequest<'a> for TypedHeader<T> {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Self::internal_from_request(req).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handler,
        test::TestClient,
        web::headers::{ContentLength, Host},
    };

    #[tokio::test]
    async fn test_typed_header_extractor() {
        #[handler(internal)]
        async fn index(content_length: TypedHeader<ContentLength>) {
            assert_eq!(content_length.0 .0, 3);
        }

        let cli = TestClient::new(index);
        let resp = cli
            .get("/")
            .header("content-length", 3)
            .body("abc")
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_typed_header_extractor_error() {
        let (req, mut body) = Request::builder().body("abc").split();
        let res = TypedHeader::<Host>::from_request(&req, &mut body).await;

        match res.unwrap_err().downcast_ref::<ParseTypedHeaderError>() {
            Some(ParseTypedHeaderError::HeaderRequired(name)) if name == "host" => {}
            _ => panic!(),
        }
    }
}
