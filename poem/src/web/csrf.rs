use std::{ops::Deref, sync::Arc};

use base64::engine::{general_purpose::STANDARD, Engine};
use libcsrf::{AesGcmCsrfProtection, CsrfProtection, UnencryptedCsrfCookie};

use crate::{FromRequest, Request, RequestBody, Result};

/// A CSRF Token for the next request.
///
/// See also [`Csrf`](crate::middleware::Csrf)
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CsrfToken(pub String);

impl Deref for CsrfToken {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a CsrfToken {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<CsrfToken>()
            .expect("To use the `CsrfToken` extractor, the `Csrf` middleware is required."))
    }
}

/// A verifier for CSRF Token.
///
/// See also [`Csrf`](crate::middleware::Csrf)
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
pub struct CsrfVerifier {
    cookie: Option<UnencryptedCsrfCookie>,
    protect: Arc<AesGcmCsrfProtection>,
}

impl CsrfVerifier {
    pub(crate) fn new(
        cookie: Option<UnencryptedCsrfCookie>,
        protect: Arc<AesGcmCsrfProtection>,
    ) -> Self {
        Self { cookie, protect }
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a CsrfVerifier {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<CsrfVerifier>()
            .expect("To use the `CsrfVerifier` extractor, the `Csrf` middleware is required."))
    }
}

impl CsrfVerifier {
    /// Return `true` if the token is valid.
    pub fn is_valid(&self, token: &str) -> bool {
        let cookie = match &self.cookie {
            Some(cookie) => cookie,
            None => return false,
        };

        let token_data = match STANDARD.decode(token) {
            Ok(data) => data,
            Err(_) => return false,
        };

        let token = match self.protect.parse_token(&token_data) {
            Ok(token) => token,
            Err(_) => return false,
        };

        self.protect.verify_token_pair(&token, cookie)
    }
}
