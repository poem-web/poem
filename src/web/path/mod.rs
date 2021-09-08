mod de;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{error::ErrorInvalidPathParams, FromRequest, Request, RequestBody, Result};

/// An extractor that will get captures from the URL and parse them using
/// `serde`.
///
/// # Example
///
/// ```
/// use poem::{handler, route, route::get, web::Path};
///
/// #[handler]
/// async fn users_teams_show(Path((user_id, team_id)): Path<(String, String)>) {
///     // ...
/// }
///
/// let mut app = route().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::{handler, route, route::get, web::Path};
///
/// #[handler]
/// async fn user_info(Path(user_id): Path<String>) {
///     // ...
/// }
///
/// let mut app = route().at("/users/:user_id", get(user_info));
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::{handler, route, route::get, web::Path};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Params {
///     user_id: String,
///     team_id: String,
/// }
///
/// #[handler]
/// async fn users_teams_show(Path(Params { user_id, team_id }): Path<Params>) {
///     // ...
/// }
///
/// let mut app = route();
/// app.at("/users/:user_id/team/:team_id", get(users_teams_show));
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
    type Error = ErrorInvalidPathParams;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        T::deserialize(de::PathDeserializer::new(&req.state().match_params))
            .map_err(|_| ErrorInvalidPathParams)
            .map(Path)
    }
}
