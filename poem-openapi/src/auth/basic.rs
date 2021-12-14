use poem::{Request, Result};
use typed_headers::{AuthScheme, Authorization, HeaderMapExt};

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
        if let Some(auth) = req.headers().typed_get::<Authorization>().ok().flatten() {
            if auth.0.scheme() == &AuthScheme::BASIC {
                if let Some(token68) = auth.token68() {
                    if let Ok(value) = base64::decode(token68.as_str()) {
                        if let Ok(value) = String::from_utf8(value) {
                            let mut s = value.split(':');
                            if let (Some(username), Some(password), None) =
                                (s.next(), s.next(), s.next())
                            {
                                return Ok(Basic {
                                    username: username.to_string(),
                                    password: password.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Err(AuthorizationError.into())
    }
}
