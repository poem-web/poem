use std::ops::{Deref, DerefMut};

use http::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::ParseJsonError, http::header, web::RequestBody, FromRequest, IntoResponse, Request,
    Response, Result,
};

/// JSON extractor and response.
///
/// To extract the specified type of JSON from the body, `T` must implement
/// [`serde::Deserialize`].
///
/// # Errors
///
/// - [`ReadBodyError`](crate::error::ReadBodyError)
/// - [`ParseJsonError`]
///
/// ```
/// use poem::{
///     handler,
///     http::{header, Method, StatusCode},
///     post,
///     test::TestClient,
///     web::Json,
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
/// async fn index(Json(user): Json<User>) -> String {
///     format!("welcome {}!", user.name)
/// }
///
/// let app = Route::new().at("/", post(index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .post("/")
///     .header(header::CONTENT_TYPE, "application/json")
///     .body(r#"{"name": "foo"}"#)
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("welcome foo!").await;
/// # });
/// ```
///
/// # Response
///
/// To serialize the specified type to JSON, `T` must implement
/// [`serde::Serialize`].
///
/// ```
/// use poem::{
///     get, handler, http::StatusCode, test::TestClient, web::Json, Endpoint, Request, Route,
/// };
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
/// }
///
/// #[handler]
/// async fn index() -> Json<User> {
///     Json(User {
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
/// resp.assert_text(r#"{"name":"foo"}"#).await;
/// # });
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: DeserializeOwned> FromRequest<'a> for Json<T> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|content_type| content_type.to_str().ok())
            .ok_or(ParseJsonError::ContentTypeRequired)?;
        if !is_json_content_type(content_type) {
            return Err(ParseJsonError::InvalidContentType(content_type.into()).into());
        }

        #[cfg(not(feature = "sonic-rs"))]
        {
            Ok(Self(
                serde_json::from_slice(&body.take()?.into_bytes().await?)
                    .map_err(ParseJsonError::Parse)?,
            ))
        }
        #[cfg(feature = "sonic-rs")]
        {
            Ok(Self(
                sonic_rs::from_slice(&body.take()?.into_bytes().await?)
                    .map_err(ParseJsonError::Parse)?,
            ))
        }
    }
}

fn is_json_content_type(content_type: &str) -> bool {
    matches!(content_type.parse::<mime::Mime>(), 
        Ok(content_type) if content_type.type_() == "application" 
        && (content_type.subtype() == "json"
        || content_type
            .suffix()
            .map_or(false, |v| v == "json")))
}

impl<T: Serialize + Send> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        #[cfg(not(feature = "sonic-rs"))]
        let vec = serde_json::to_vec(&self.0);
        #[cfg(feature = "sonic-rs")]
        let vec = sonic_rs::to_vec(&self.0);

        let data = match vec {
            Ok(data) => data,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(err.to_string())
            }
        };
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .body(data)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    #[cfg(not(feature = "sonic-rs"))]
    use serde_json::{json, to_string};
    #[cfg(feature = "sonic-rs")]
    use sonic_rs::{json, to_string};

    use super::*;
    use crate::{handler, test::TestClient};

    #[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
    struct CreateResource {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_json_extractor() {
        #[handler(internal)]
        async fn index(query: Json<CreateResource>) {
            assert_eq!(query.name, "abc");
            assert_eq!(query.value, 100);
        }

        let cli = TestClient::new(index);
        cli.post("/")
            .body_json(&json!({"name": "abc", "value": 100})) // body_json has already set request with `application/json` content type
            .send()
            .await
            .assert_status_is_ok();
    }
    #[tokio::test]
    async fn test_json_extractor_fail() {
        #[handler(internal)]
        async fn index(query: Json<CreateResource>) {
            assert_eq!(query.name, "abc");
            assert_eq!(query.value, 100);
        }
        let create_resource = CreateResource {
            name: "abc".to_string(),
            value: 100,
        };
        let cli = TestClient::new(index);
        cli.post("/")
            // .header(header::CONTENT_TYPE, "application/json")
            .body(to_string(&create_resource).expect("Invalid json"))
            .send()
            .await
            .assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_json_response() {
        #[handler(internal)]
        async fn index() -> Json<CreateResource> {
            Json(CreateResource {
                name: "abc".to_string(),
                value: 100,
            })
        }

        let cli = TestClient::new(index);
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_json(&CreateResource {
            name: "abc".to_string(),
            value: 100,
        })
        .await;
    }
}
