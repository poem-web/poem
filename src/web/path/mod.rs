mod de;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{FromRequest, Request, RequestBody, Result};

define_simple_errors!(
    /// Only the endpoints under the router can get the path parameters, otherwise this error will occur.
    (ErrorInvalidPathParams, INTERNAL_SERVER_ERROR, "invalid path params");
);

/// An extractor that will get captures from the URL and parse them using
/// `serde`.
///
/// # Example
///
/// ```
/// use poem::{get, route, web::Path};
///
/// #[get]
/// async fn users_teams_show(Path((user_id, team_id)): Path<(String, String)>) {
///     // ...
/// }
///
/// let app = route().at("/users/:user_id/team/:team_id", users_teams_show);
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::{get, route, web::Path};
///
/// #[get]
/// async fn user_info(Path(user_id): Path<String>) {
///     // ...
/// }
///
/// let app = route().at("/users/:user_id", user_info);
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::{get, route, web::Path};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Params {
///     user_id: String,
///     team_id: String,
/// }
///
/// #[get]
/// async fn users_teams_show(Path(Params { user_id, team_id }): Path<Params>) {
///     // ...
/// }
///
/// let app = route().at("/users/:user_id/team/:team_id", users_teams_show);
/// ```
#[derive(Debug)]
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
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        T::deserialize(de::PathDeserializer::new(&req.state().match_params))
            .map_err(|_| ErrorInvalidPathParams.into())
            .map(Path)
    }
}
