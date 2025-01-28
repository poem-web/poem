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
#[derive(Clone)]
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
pub struct CsrfVerifier {
    cookie: Option<UnencryptedCsrfCookie>,
    protect: Arc<AesGcmCsrfProtection>,
}

/// Enum representing CSRF validation error
#[derive(Clone, thiserror::Error, Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
pub enum CsrfError {
    #[error("CSRF cookie missing")]
    MissingCookie,
    #[error("CSRF cookie has invalid base64 value")]
    CannotBeDecoded,
    #[error(transparent)]
    Inner(#[from] libcsrf::CsrfError),
}

impl CsrfVerifier {
    pub(crate) fn new(
        cookie: Option<UnencryptedCsrfCookie>,
        protect: Arc<AesGcmCsrfProtection>,
    ) -> Self {
        Self { cookie, protect }
    }
}

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
        self.validate(token).is_ok()
    }

    /// Validates csrf token and returns error with description of what failed.
    /// If you want simplified version of this method that returns boolean,
    /// use [`Self::is_valid`]
    pub fn validate(&self, token: &str) -> Result<(), CsrfError> {
        let cookie = match &self.cookie {
            Some(cookie) => cookie,
            None => return Err(CsrfError::MissingCookie),
        };

        let token_data = match STANDARD.decode(token) {
            Ok(data) => data,
            Err(_) => return Err(CsrfError::CannotBeDecoded),
        };

        let token = self.protect.parse_token(&token_data)?;

        self.protect
            .verify_token_pair(&token, cookie)
            .map_err(Into::into)
    }
}
