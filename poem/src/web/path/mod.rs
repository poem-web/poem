mod de;

use std::ops::{Deref, DerefMut};

pub(crate) use de::PathDeserializer;
use serde::de::DeserializeOwned;

use crate::{FromRequest, Request, RequestBody, Result, error::ParsePathError};

/// An extractor that will get captures from the URL and parse them using
/// `serde`.
///
/// # Errors
///
/// - [`ParsePathError`]
///
/// # Example
///
/// ```
/// use poem::{
///     Endpoint, Request, Route, get, handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     web::Path,
/// };
///
/// #[handler]
/// async fn users_teams_show(Path((user_id, team_id)): Path<(String, String)>) -> String {
///     format!("{}:{}", user_id, team_id)
/// }
///
/// let app = Route::new().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/users/100/team/300").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("100:300").await;
/// # });
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::{
///     Endpoint, Request, Route, get, handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     web::Path,
/// };
///
/// #[handler]
/// async fn user_info(Path(user_id): Path<String>) -> String {
///     user_id
/// }
///
/// let app = Route::new().at("/users/:user_id", get(user_info));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/users/100").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("100").await;
/// # });
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::{
///     Endpoint, Request, Route, get, handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     web::Path,
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
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/users/foo/team/100").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("foo:100").await;
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

impl<T: DeserializeOwned> Path<T> {
    async fn internal_from_request(req: &Request) -> Result<Self, ParsePathError> {
        Ok(Path(
            T::deserialize(de::PathDeserializer::new(&req.state().match_params))
                .map_err(|_| ParsePathError)?,
        ))
    }
}

impl<'a, T: DeserializeOwned> FromRequest<'a> for Path<T> {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Self::internal_from_request(req).await.map_err(Into::into)
    }
}
