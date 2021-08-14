mod de;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::error::{Error, ErrorInvalidPathParams, ErrorMissingRouteParams, Result};
use crate::request::Request;
use crate::route_recognizer::Params;
use crate::web::FromRequest;

/// An extractor that will get captures from the URL and parse them using `serde`.
///
/// # Example
///
/// ```
/// use poem::web::Path;
/// use poem::prelude::*;
///
/// async fn users_teams_show(
///     Path((user_id, team_id)): Path<(String, String)>,
/// ) {
///     // ...
/// }
///
/// let app = route().at("/users/:user_id/team/:team_id", get(users_teams_show));
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```
/// use poem::web::Path;
/// use poem::prelude::*;
///
/// async fn user_info(Path(user_id): Path<String>) {
///     // ...
/// }
///
/// let app = route().at("/users/:user_id", get(user_info));
/// ```
///
/// Path segments also can be deserialized into any type that implements [`serde::Deserialize`](https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html).
/// Path segment labels will be matched with struct field names.
///
/// ```
/// use poem::web::Path;
/// use poem::prelude::*;
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
/// let app = route().at("/users/:user_id/team/:team_id", get(users_teams_show));
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
            .ok_or_else(|| Into::<Error>::into(ErrorMissingRouteParams))?;
        T::deserialize(de::PathDeserializer::new(params))
            .map_err(|_| ErrorInvalidPathParams.into())
            .map(Path)
    }
}
