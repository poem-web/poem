use poem::{
    Request, Result,
    web::headers::{Authorization, HeaderMapExt},
};

use crate::{auth::BearerAuthorization, error::AuthorizationError};

/// Used to extract the token68 from the request.
pub struct Bearer {
    /// token
    pub token: String,
}

impl BearerAuthorization for Bearer {
    fn from_request(req: &Request) -> Result<Self> {
        if let Some(auth) = req
            .headers()
            .typed_get::<Authorization<poem::web::headers::authorization::Bearer>>()
        {
            return Ok(Bearer {
                token: auth.token().to_string(),
            });
        }

        Err(AuthorizationError.into())
    }
}
