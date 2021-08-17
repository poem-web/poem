use std::sync::Arc;

use http::HeaderValue;
use parking_lot::Mutex;

use crate::{
    error::{ErrorCookieIllegal, ErrorNoCookie},
    http::{header, HeaderMap},
    Body, FromRequest, Request, Result,
};

/// Representation of an HTTP cookie.
pub type Cookie = cookie::Cookie<'static>;

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Cookie {
    async fn from_request(req: &'a Request, _body: &mut Option<Body>) -> Result<Self> {
        let value = req.headers().get(header::COOKIE).ok_or(ErrorNoCookie)?;
        let value = value.to_str().map_err(|_| ErrorCookieIllegal)?;
        let cookie = cookie::Cookie::parse(value.to_string()).map_err(|_| ErrorCookieIllegal)?;
        Ok(cookie)
    }
}

/// A collection of cookies that tracks its modifications.
#[derive(Default, Clone)]
pub struct CookieJar(pub(crate) Arc<Mutex<::cookie::CookieJar>>);

impl CookieJar {
    /// Adds cookie to this jar. If a cookie with the same name already exists,
    /// it is replaced with cookie.
    pub fn add(&mut self, cookie: Cookie) {
        self.0.lock().add(cookie);
    }

    /// Removes cookie from this jar.
    pub fn remove(&mut self, cookie: Cookie) {
        self.0.lock().remove(cookie);
    }

    /// Returns a reference to the [`Cookie`] inside this jar with the `name`.
    /// If no such cookie exists, returns `None`.
    pub fn get(&mut self, name: &str) -> Option<Cookie> {
        self.0.lock().get(name).cloned()
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a CookieJar {
    async fn from_request(req: &'a Request, _body: &mut Option<Body>) -> Result<Self> {
        Ok(req.cookie())
    }
}

impl CookieJar {
    pub(crate) fn append_delta_to_headers(&self, headers: &mut HeaderMap) {
        let cookie = self.0.lock();
        for cookie in cookie.delta() {
            let value = cookie.to_string();
            if let Ok(value) = HeaderValue::from_str(&value) {
                headers.append(header::SET_COOKIE, value);
            }
        }
    }
}
