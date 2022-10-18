use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{
    error::ParseFormError,
    http::{
        header::{self},
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
/// - [`ReadBodyError`](crate::error::ReadBodyError)
/// - [`ParseFormError`]
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{Method, StatusCode, Uri},
///     test::TestClient,
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
/// let app = Route::new().at("/", get(index).post(index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .get("/")
///     .query("title", &"foo")
///     .query("content", &"bar")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("foo:bar").await;
///
/// let resp = cli
///     .post("/")
///     .form(&[("title", "foo"), ("content", "bar")])
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("foo:bar").await;
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
            let content_type = req
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|content_type| content_type.to_str().ok())
                .ok_or(ParseFormError::ContentTypeRequired)?;
            if !is_form_content_type(content_type) {
                return Err(ParseFormError::InvalidContentType(content_type.into()).into());
            }

            Ok(Self(
                serde_urlencoded::from_bytes(&body.take()?.into_vec().await?)
                    .map_err(ParseFormError::UrlDecode)?,
            ))
        }
    }
}

fn is_form_content_type(content_type: &str) -> bool {
    matches!(content_type.parse::<mime::Mime>(), 
        Ok(content_type) if content_type.type_() == "application" 
        && (content_type.subtype() == "x-www-form-urlencoded"
        || content_type
            .suffix()
            .map_or(false, |v| v == "x-www-form-urlencoded")))
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use serde::Deserialize;

    use super::*;
    use crate::{handler, test::TestClient};

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

        let cli = TestClient::new(index);

        cli.get("/")
            .query("name", &"abc")
            .query("value", &"100")
            .send()
            .await
            .assert_status_is_ok();

        cli.post("/")
            .form(&[("name", "abc"), ("value", "100")])
            .send()
            .await
            .assert_status_is_ok();

        cli.post("/")
            .content_type("application/json")
            .body("name=abc&value=100")
            .send()
            .await
            .assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
}
