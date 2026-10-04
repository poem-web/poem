use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{
    FromRequest, Request, Result,
    error::ParseFormError,
    http::{
        Method,
        header::{self},
    },
    web::RequestBody,
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
///     Endpoint, Request, Route, get, handler,
///     http::{Method, StatusCode, Uri},
///     test::TestClient,
///     web::Form,
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
                serde_urlencoded::from_bytes(&body.take()?.into_bytes().await?)
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
            .is_some_and(|v| v == "x-www-form-urlencoded")))
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

    #[tokio::test]
    async fn form_preserves_fragments_and_errors() {
        use bytes::Bytes;

        use crate::{Body, error::ReadBodyError};

        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            text: String,
            number: u32,
        }

        let req = Request::builder()
            .method(Method::POST)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .finish();
        let stream = futures_util::stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(b"text=%E4%")),
            Ok(Bytes::from_static(b"BD%A0&number=42")),
        ]);
        let mut body = RequestBody::new(Body::from_bytes_stream(stream));
        assert_eq!(
            Form::<Data>::from_request(&req, &mut body).await.unwrap().0,
            Data {
                text: "你".into(),
                number: 42
            }
        );
        let err = Form::<Data>::from_request(&req, &mut body)
            .await
            .err()
            .unwrap();
        assert!(matches!(
            err.downcast_ref::<ReadBodyError>(),
            Some(ReadBodyError::BodyHasBeenTaken)
        ));

        let mut body = RequestBody::new("text=test&number=invalid".into());
        let err = Form::<Data>::from_request(&req, &mut body)
            .await
            .err()
            .unwrap();
        assert!(matches!(
            err.downcast_ref::<ParseFormError>(),
            Some(ParseFormError::UrlDecode(_))
        ));
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);

        let stream = futures_util::stream::once(async {
            Err::<Bytes, _>(std::io::Error::other("read failed"))
        });
        let mut body = RequestBody::new(Body::from_bytes_stream(stream));
        let err = Form::<Data>::from_request(&req, &mut body)
            .await
            .err()
            .unwrap();
        assert!(matches!(
            err.downcast_ref::<ReadBodyError>(),
            Some(ReadBodyError::Io(_))
        ));
    }
}
