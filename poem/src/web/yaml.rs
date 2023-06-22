use std::ops::{Deref, DerefMut};

use http::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::ParseYamlError, http::header, web::RequestBody, FromRequest, IntoResponse, Request,
    Response, Result,
};

/// JSON extractor and response.
///
/// To extract the specified type of YAML from the body, `T` must implement
/// [`serde::Deserialize`].
///
/// # Errors
///
/// - [`ReadBodyError`](crate::error::ReadBodyError)
/// - [`ParseYamlError`]
///
/// ```
/// use poem::{
///     handler,
///     http::{header, Method, StatusCode},
///     post,
///     test::TestClient,
///     web::Yaml,
///     Endpoint, Request, Route,
/// };
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct User {
///     name: String,
/// }
///
/// #[handler]
/// async fn index(Yaml(user): Yaml<User>) -> String {
///     format!("welcome {}!", user.name)
/// }
///
/// let app = Route::new().at("/", post(index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .post("/")
///     .header(header::CONTENT_TYPE, "application/yaml")
///     .body(r#"name: foo"#)
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("welcome foo!").await;
/// # });
/// ```
///
/// # Response
///
/// To serialize the specified type to YAML, `T` must implement
/// [`serde::Serialize`].
///
/// ```
/// use poem::{
///     get, handler, http::StatusCode, test::TestClient, web::Yaml, Endpoint, Request, Route,
/// };
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
/// }
///
/// #[handler]
/// async fn index() -> Yaml<User> {
///     Yaml(User {
///         name: "foo".to_string(),
///     })
/// }
///
/// let app = Route::new().at("/", get(index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text(
///     r#"name: foo
/// "#,
/// )
/// .await;
/// # });
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Yaml<T>(pub T);

impl<T> Deref for Yaml<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Yaml<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Yaml<T> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|content_type| content_type.to_str().ok())
            .ok_or(ParseYamlError::ContentTypeRequired)?;
        if !is_yaml_content_type(content_type) {
            return Err(ParseYamlError::InvalidContentType(content_type.into()).into());
        }

        Ok(Self(
            serde_yaml::from_slice(&body.take()?.into_bytes().await?)
                .map_err(ParseYamlError::Parse)?,
        ))
    }
}

fn is_yaml_content_type(content_type: &str) -> bool {
    matches!(content_type.parse::<mime::Mime>(),
        Ok(content_type) if content_type.type_() == "application"
        && (content_type.subtype() == "yaml"
        || content_type
            .suffix()
            .map_or(false, |v| v == "yaml")))
}

impl<T: Serialize + Send> IntoResponse for Yaml<T> {
    fn into_response(self) -> Response {
        let data = match serde_yaml::to_string(&self.0) {
            Ok(data) => data,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(err.to_string())
            }
        };
        Response::builder()
            .header(header::CONTENT_TYPE, "application/yaml; charset=utf-8")
            .body(data)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{handler, test::TestClient};

    #[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
    struct CreateResource {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_yaml_extractor() {
        #[handler(internal)]
        async fn index(query: Yaml<CreateResource>) {
            assert_eq!(query.name, "abc");
            assert_eq!(query.value, 100);
        }

        let cli = TestClient::new(index);
        cli.post("/")
            .body_yaml(&CreateResource {
                name: "abc".to_string(),
                value: 100,
            }) // body_yaml has already set request with `application/yaml` content type
            .send()
            .await
            .assert_status_is_ok();
    }
    #[tokio::test]
    async fn test_yaml_extractor_fail() {
        #[handler(internal)]
        async fn index(query: Yaml<CreateResource>) {
            assert_eq!(query.name, "abc");
            assert_eq!(query.value, 100);
        }
        let create_resource = CreateResource {
            name: "abc".to_string(),
            value: 100,
        };
        let cli = TestClient::new(index);
        cli.post("/")
            .body(serde_yaml::to_string(&create_resource).expect("Invalid yaml"))
            .send()
            .await
            .assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_yaml_response() {
        #[handler(internal)]
        async fn index() -> Yaml<CreateResource> {
            Yaml(CreateResource {
                name: "abc".to_string(),
                value: 100,
            })
        }

        let cli = TestClient::new(index);
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_yaml(&CreateResource {
            name: "abc".to_string(),
            value: 100,
        })
        .await;
    }
}
