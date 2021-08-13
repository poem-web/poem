mod de;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::error::{ErrorInvalidPathParams, ErrorMissingRouteParams};
use crate::route_recognizer::Params;
use crate::{Error, FromRequest, Request, Result};

/// An extractor that will get captures from the URL and parse them using `serde`.
///
/// # Example
///
/// ```
/// use poem::web::Path;
/// use poem::route::{Route, get};
///
/// async fn users_teams_show(
///     Path((user_id, team_id)): Path<(String, String)>,
/// ) {
///     // ...
/// }
///
/// let route = Route::new().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::web::Path;
/// use poem::route::{Route, get};
///
/// async fn user_info(Path(user_id): Path<String>) {
///     // ...
/// }
///
/// let route = Route::new().at("/users/:user_id", get(user_info));
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::web::Path;
/// use poem::route::{Route, get};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Params {
///     user_id: String,
///     team_id: String,
/// }
///
/// async fn users_teams_show(
///     Path(Params { user_id, team_id }): Path<Params>,
/// ) {
///     // ...
/// }
///
/// let route = Route::new().at("/users/:user_id/team/:team_id", get(users_teams_show));
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
impl<T> FromRequest for Path<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(req: &mut Request) -> Result<Self> {
        let params = req
            .extensions_mut()
            .get::<Params>()
            .ok_or_else(|| Error::internal_server_error(ErrorMissingRouteParams))?;
        T::deserialize(de::PathDeserializer::new(params))
            .map_err(|_| Error::internal_server_error(ErrorInvalidPathParams))
            .map(Path)
    }
}
