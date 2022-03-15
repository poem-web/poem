use poem::{
    web::headers::{Authorization, HeaderMapExt},
    Request, Result,
};

use crate::{auth::BasicAuthorization, error::AuthorizationError};

/// Used to extract the username/password from the request.
pub struct Basic {
    /// username
    pub username: String,

    /// password
    pub password: String,
}

impl BasicAuthorization for Basic {
    fn from_request(req: &Request) -> Result<Self> {
        if let Some(auth) = req
            .headers()
            .typed_get::<Authorization<poem::web::headers::authorization::Basic>>()
        {
            return Ok(Basic {
                username: auth.username().to_string(),
                password: auth.password().to_string(),
            });
        }

        Err(AuthorizationError.into())
    }
}
