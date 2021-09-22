use std::{convert::Infallible, str::FromStr, sync::Arc};

use http::HeaderValue;
use parking_lot::Mutex;

use crate::{
    error::ParseCookieError,
    http::{header, HeaderMap},
    FromRequest, Request, RequestBody, Result,
};

/// Representation of an HTTP cookie.
pub type Cookie = cookie::Cookie<'static>;

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Cookie {
    type Error = ParseCookieError;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        let value = req
            .headers()
            .get(header::COOKIE)
            .ok_or(ParseCookieError::CookieHeaderRequired)?;
        let value = value
            .to_str()
            .map_err(|_| ParseCookieError::CookieIllegal)?;
        let cookie = cookie::Cookie::parse(value.to_string())
            .map_err(|_| ParseCookieError::CookieIllegal)?;
        Ok(cookie)
    }
}

/// A collection of cookies that tracks its modifications.
///
/// NOTE: To use the `CookieJar` extractor, the
/// [`CookieJarManager`](crate::middleware::CookieJarManager) middleware is
/// required.
#[derive(Default, Clone)]
pub struct CookieJar(pub(crate) Arc<Mutex<::cookie::CookieJar>>);

impl CookieJar {
    /// Adds cookie to this jar. If a cookie with the same name already exists,
    /// it is replaced with cookie.
    pub fn add(&self, cookie: Cookie) {
        self.0.lock().add(cookie);
    }

    /// Removes cookie from this jar.
    pub fn remove(&self, cookie: Cookie) {
        self.0.lock().remove(cookie);
    }

    /// Returns a reference to the [`Cookie`] inside this jar with the `name`.
    /// If no such cookie exists, returns `None`.
    pub fn get(&self, name: &str) -> Option<Cookie> {
        self.0.lock().get(name).cloned()
    }

    /// Removes all delta cookies.
    pub fn reset_delta(&self) {
        self.0.lock().reset_delta();
    }
}

impl FromStr for CookieJar {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cookie_jar = ::cookie::CookieJar::new();

        for cookie_str in s.split(';').map(str::trim) {
            if let Ok(cookie) = ::cookie::Cookie::parse_encoded(cookie_str) {
                cookie_jar.add_original(cookie.into_owned());
            }
        }

        Ok(CookieJar(Arc::new(Mutex::new(cookie_jar))))
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a CookieJar {
    type Error = Infallible;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        Ok(req.cookie())
    }
}

impl CookieJar {
    pub(crate) fn extract_from_headers(headers: &HeaderMap) -> Self {
        headers
            .get(header::COOKIE)
            .and_then(|value| std::str::from_utf8(value.as_bytes()).ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or_default()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_jar() {
        let a = Cookie::new("a", 100.to_string());
        let b = Cookie::new("b", 200.to_string());
        let c = Cookie::new("c", 300.to_string());

        let cookie_str = format!("{}; {}", a, b);

        let cookie_jar = CookieJar::from_str(&cookie_str).unwrap();
        assert_eq!(cookie_jar.get("a").unwrap(), a);
        assert_eq!(cookie_jar.get("b").unwrap(), b);

        // add cookie c
        {
            cookie_jar.add(c.clone());

            let mut headers = HeaderMap::new();
            cookie_jar.append_delta_to_headers(&mut headers);

            let mut values = headers.get_all(header::SET_COOKIE).into_iter();
            assert_eq!(
                values.next().unwrap(),
                &HeaderValue::from_str(&c.to_string()).unwrap()
            );
            assert!(values.next().is_none());
        }

        // remove cookie a
        {
            cookie_jar.reset_delta();
            cookie_jar.remove(a.clone());

            let mut headers = HeaderMap::new();
            cookie_jar.append_delta_to_headers(&mut headers);

            let mut values = headers.get_all(header::SET_COOKIE).into_iter();
            let value = values.next().unwrap();
            let remove_c = Cookie::parse(value.to_str().unwrap().to_string()).unwrap();
            assert_eq!(remove_c.name(), "a");
            assert_eq!(remove_c.value(), "");

            assert!(values.next().is_none());
        }
    }

    #[tokio::test]
    async fn test_cookie_extractor() {
        let req = Request::builder()
            .header(header::COOKIE, Cookie::new("a", "1").to_string())
            .finish();
        let (req, mut body) = req.split();
        let cookie = Cookie::from_request(&req, &mut body).await.unwrap();
        assert_eq!(cookie.name(), "a");
        assert_eq!(cookie.value(), "1");
    }
}
