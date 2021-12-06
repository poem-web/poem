mod de;

use std::ops::{Deref, DerefMut};

pub(crate) use de::PathDeserializer;
use serde::de::DeserializeOwned;

use crate::{error::ParsePathError, FromRequest, Request, RequestBody, Result};

/// An extractor that will get captures from the URL and parse them using
/// `serde`.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{StatusCode, Uri},
///     web::Path,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// async fn users_teams_show(Path((user_id, team_id)): Path<(String, String)>) -> String {
///     format!("{}:{}", user_id, team_id)
/// }
///
/// let app = Route::new().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app
///     .call(
///         Request::builder()
///             .uri(Uri::from_static("/users/100/team/300"))
///             .finish(),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "100:300");
/// # });
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::{
///     get, handler,
///     http::{StatusCode, Uri},
///     web::Path,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// async fn user_info(Path(user_id): Path<String>) -> String {
///     user_id
/// }
///
/// let app = Route::new().at("/users/:user_id", get(user_info));
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app
///     .call(
///         Request::builder()
///             .uri(Uri::from_static("/users/100"))
///             .finish(),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "100");
/// # });
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::{
///     get, handler,
///     http::{StatusCode, Uri},
///     web::Path,
///     Endpoint, Request, Route,
/// };
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Params {
///     user_id: String,
///     team_id: String,
/// }
///
/// #[handler]
/// async fn users_teams_show(Path(Params { user_id, team_id }): Path<Params>) -> String {
///     format!("{}:{}", user_id, team_id)
/// }
///
/// let app = Route::new().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app
///     .call(
///         Request::builder()
///             .uri(Uri::from_static("/users/foo/team/100"))
///             .finish(),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "foo:100");
/// # });
/// ```
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Path<T>(pub T);

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Path<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Path<T> {
    type Error = ParsePathError;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        T::deserialize(de::PathDeserializer::new(&req.state().match_params))
            .map_err(|_| ParsePathError)
            .map(Path)
    }
}
