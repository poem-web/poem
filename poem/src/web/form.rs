use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{
    error::ParseFormError,
    http::{
        header::{self, HeaderValue},
        Method,
    },
    web::RequestBody,
    FromRequest, Request, Result,
};

/// An extractor that can deserialize some type from query string or body.
///
/// If the method is not `GET`, the query parameters will be parsed from the
/// body, otherwise it is like [`Query`](crate::web::Query).
///
/// If the `Content-Type` is not `application/x-www-form-urlencoded`, then a
/// `Bad Request` response will be returned.
///
/// # Errors
///
/// - [`ReadBodyError`]
/// - [`ParseFormError`]
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{Method, StatusCode, Uri},
///     web::Form,
///     Endpoint, Request, Route,
/// };
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct CreateDocument {
///     title: String,
///     content: String,
/// }
///
/// #[handler]
/// fn index(Form(CreateDocument { title, content }): Form<CreateDocument>) -> String {
///     format!("{}:{}", title, content)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new().at("/", get(index).post(index));
///
/// let resp = app
///     .call(
///         Request::builder()
///             .uri(Uri::from_static("/?title=foo&content=bar"))
///             .finish(),
///     )
///     .await
///     .unwrap();
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "foo:bar");
///
/// let resp = app
///     .call(
///         Request::builder()
///             .method(Method::POST)
///             .uri(Uri::from_static("/"))
///             .content_type("application/x-www-form-urlencoded")
///             .body("title=foo&content=bar"),
///     )
///     .await
///     .unwrap();
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "foo:bar");
/// # });
/// ```
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

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Form<T> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        if req.method() == Method::GET {
            Ok(
                serde_urlencoded::from_str(req.uri().query().unwrap_or_default())
                    .map_err(ParseFormError::UrlDecode)
                    .map(Self)?,
            )
        } else {
            let content_type = req.headers().get(header::CONTENT_TYPE);
            if content_type
                != Some(&HeaderValue::from_static(
                    "application/x-www-form-urlencoded",
                ))
            {
                return match content_type.and_then(|value| value.to_str().ok()) {
                    Some(ty) => Err(ParseFormError::InvalidContentType(ty.to_string()).into()),
                    None => Err(ParseFormError::ContentTypeRequired.into()),
                };
            }

            Ok(Self(
                serde_urlencoded::from_bytes(&body.take()?.into_vec().await?)
                    .map_err(ParseFormError::UrlDecode)?,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::{handler, http::Uri, Endpoint};

    #[tokio::test]
    async fn test_form_extractor() {
        #[derive(Deserialize)]
        struct CreateResource {
            name: String,
            value: i32,
        }

        #[handler(internal)]
        async fn index(form: Form<CreateResource>) {
            assert_eq!(form.name, "abc");
            assert_eq!(form.value, 100);
        }

        index
            .call(
                Request::builder()
                    .uri(Uri::from_static("/?name=abc&value=100"))
                    .finish(),
            )
            .await
            .unwrap();

        index
            .call(
                Request::builder()
                    .method(Method::POST)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body("name=abc&value=100"),
            )
            .await
            .unwrap();

        assert!(index
            .call(
                Request::builder()
                    .method(Method::POST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body("name=abc&value=100"),
            )
            .await
            .unwrap_err()
            .is::<ParseFormError>());
    }
}
