use std::ops::{Deref, DerefMut};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::ParseJsonError, http::header, web::RequestBody, Error, FromRequest, IntoResponse,
    Request, Response, Result,
};

/// JSON extractor and response.
///
/// # Extractor
///
/// To extract the specified type of JSON from the body, `T` must implement
/// [`serde::Deserialize`].
///
/// ```
/// use poem::{
///     handler,
///     http::{Method, StatusCode},
///     post,
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
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new().at("/", post(index));
/// let resp = app
///     .call(
///         Request::builder()
///             .method(Method::POST)
///             .body(r#"{"name": "foo"}"#),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(
///     resp.into_body().into_string().await.unwrap(),
///     "welcome foo!"
/// );
/// # });
/// ```
///
/// # Response
///
/// To serialize the specified type to JSON, `T` must implement
/// [`serde::Serialize`].
///
/// ```
/// use poem::{get, handler, http::StatusCode, web::Json, Endpoint, Request, Route};
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
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new().at("/", get(index));
/// let resp = app.call(Request::default()).await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(
///     resp.into_body().into_string().await.unwrap(),
///     r#"{"name":"foo"}"#
/// )
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

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Json<T> {
    type Error = ParseJsonError;

    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self, Self::Error> {
        let data = body.take()?.into_bytes().await?;
        Ok(Self(serde_json::from_slice(&data)?))
    }
}

impl<T: Serialize + Send> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        let data = match serde_json::to_vec(&self.0) {
            Ok(data) => data,
            Err(err) => return Error::internal_server_error(err).as_response(),
        };
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(data)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{
        handler,
        http::{Method, StatusCode},
        Endpoint,
    };

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

        index
            .call(
                Request::builder()
                    .method(Method::POST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(
                        r#"
                    {
                        "name": "abc",
                        "value": 100
                    }
                    "#,
                    ),
            )
            .await;
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

        let mut resp = index.call(Request::default()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            serde_json::from_str::<CreateResource>(&resp.take_body().into_string().await.unwrap())
                .unwrap(),
            CreateResource {
                name: "abc".to_string(),
                value: 100,
            }
        );
    }
}
