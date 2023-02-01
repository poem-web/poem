//! Cookie related types.

use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, TimeZone, Utc};
use http::HeaderValue;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    error::ParseCookieError,
    http::{header, HeaderMap},
    FromRequest, Request, RequestBody, Result,
};

/// The `SameSite` cookie attribute.
pub type SameSite = libcookie::SameSite;

/// HTTP cookie extractor.
///
/// # Errors
///
/// - [`ParseCookieError`]
#[derive(Clone, Debug, PartialEq)]
pub struct Cookie(libcookie::Cookie<'static>);

impl Display for Cookie {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.encoded().fmt(f)
    }
}

impl Cookie {
    /// Creates a new Cookie with the given `name` and serialized `value`.
    pub fn new(name: impl Into<String>, value: impl Serialize) -> Self {
        Self(libcookie::Cookie::new(
            name.into(),
            serde_json::to_string(&value).unwrap_or_default(),
        ))
    }

    /// Creates a new Cookie with the given `name` and `value`.
    pub fn new_with_str(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self(libcookie::Cookie::new(name.into(), value.into()))
    }

    /// Creates a new `Cookie` with the given name and an empty value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::named("foo");
    /// assert_eq!(cookie.name(), "foo");
    /// assert!(cookie.value_str().is_empty());
    /// ```
    pub fn named(name: impl Into<String>) -> Self {
        Self::new_with_str(name, "")
    }

    /// Parses a Cookie from the given HTTP cookie header value string.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ParseCookieError> {
        Ok(Self(
            libcookie::Cookie::parse_encoded(s.as_ref().to_string())
                .map_err(|_| ParseCookieError::CookieIllegal)?,
        ))
    }

    /// Returns the Domain of the cookie if one was specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar; domain=example.com").unwrap();
    /// assert_eq!(cookie.domain(), Some("example.com"));
    /// ```
    pub fn domain(&self) -> Option<&str> {
        self.0.domain()
    }

    /// Returns the expiration date-time of the cookie if one was specified.
    pub fn expires(&self) -> Option<DateTime<Utc>> {
        self.0
            .expires_datetime()
            .and_then(|t| Utc.timestamp_opt(t.unix_timestamp(), 0).single())
    }

    /// Returns whether this cookie was marked `HttpOnly` or not.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar; HttpOnly").unwrap();
    /// assert!(cookie.http_only());
    /// ```
    pub fn http_only(&self) -> bool {
        self.0.http_only().unwrap_or_default()
    }

    /// Makes `self` a `permanent` cookie by extending its expiration and max
    /// age 20 years into the future.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use poem::web::cookie::Cookie;
    ///
    /// let mut cookie = Cookie::new_with_str("foo", "bar");
    /// cookie.make_permanent();
    /// assert_eq!(
    ///     cookie.max_age(),
    ///     Some(Duration::from_secs(60 * 60 * 24 * 365 * 20))
    /// );
    /// ```
    pub fn make_permanent(&mut self) {
        self.0.make_permanent();
    }

    /// Make `self` a `removal` cookie by clearing its value, setting a max-age
    /// of 0, and setting an expiration date far in the past.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use poem::web::cookie::Cookie;
    ///
    /// let mut cookie = Cookie::new_with_str("foo", "bar");
    /// cookie.make_removal();
    /// assert_eq!(cookie.max_age(), Some(Duration::from_secs(0)));
    /// ```
    pub fn make_removal(&mut self) {
        self.0.make_removal();
    }

    /// Returns the specified max-age of the cookie if one was specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar; Max-Age=3600").unwrap();
    /// assert_eq!(cookie.max_age(), Some(Duration::from_secs(3600)));
    /// ```
    pub fn max_age(&self) -> Option<Duration> {
        self.0.max_age().map(|d| {
            let seconds = d.whole_seconds().max(0) as u64;
            let nano_seconds = d.subsec_nanoseconds().max(0) as u32;
            Duration::new(seconds, nano_seconds)
        })
    }

    /// Returns the name of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar").unwrap();
    /// assert_eq!(cookie.name(), "foo");
    /// ```
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// Returns the `Path` of the cookie if one was specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar; path=/api").unwrap();
    /// assert_eq!(cookie.path(), Some("/api"));
    /// ```
    pub fn path(&self) -> Option<&str> {
        self.0.path()
    }

    /// Returns the `SameSite` attribute of this cookie if one was specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::{Cookie, SameSite};
    ///
    /// let cookie = Cookie::parse("foo=bar; SameSite=Strict").unwrap();
    /// assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    /// ```
    pub fn same_site(&self) -> Option<SameSite> {
        self.0.same_site()
    }

    /// Returns whether this cookie was marked `Secure` or not.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::web::cookie::Cookie;
    ///
    /// let cookie = Cookie::parse("foo=bar; Secure").unwrap();
    /// assert!(cookie.secure());
    /// ```
    pub fn secure(&self) -> bool {
        self.0.secure().unwrap_or_default()
    }

    /// Sets the `domain` of `self` to `domain`.
    pub fn set_domain(&mut self, domain: impl Into<String>) {
        self.0.set_domain(domain.into());
    }

    /// Sets the expires field of `self` to `time`.
    pub fn set_expires(&mut self, time: DateTime<impl TimeZone>) {
        self.0.set_expires(libcookie::Expiration::DateTime(
            time::OffsetDateTime::from_unix_timestamp(time.timestamp()).unwrap(),
        ));
    }

    /// Sets the value of `HttpOnly` in `self` to `value`.
    pub fn set_http_only(&mut self, value: impl Into<Option<bool>>) {
        self.0.set_http_only(value);
    }

    /// Sets the value of `MaxAge` in `self` to `value`.
    pub fn set_max_age(&mut self, value: Duration) {
        self.0.set_max_age(Some(time::Duration::new(
            value.as_secs() as i64,
            value.subsec_nanos() as i32,
        )));
    }

    /// Sets the name of `self` to `name`.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.0.set_name(name.into());
    }

    /// Sets the path of self to path.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.0.set_path(path.into());
    }

    /// Sets the value of `SameSite` in `self` to `value`.
    pub fn set_same_site(&mut self, value: impl Into<Option<SameSite>>) {
        self.0.set_same_site(value);
    }

    /// Sets the value of `Secure` in `self` to `value`.
    pub fn set_secure(&mut self, value: impl Into<Option<bool>>) {
        self.0.set_secure(value);
    }

    /// Sets the value of `self` to `value`.
    pub fn set_value_str(&mut self, value: impl Into<String>) {
        self.0.set_value(value.into());
    }

    /// Sets the value of `self` to the serialized `value`.
    pub fn set_value(&mut self, value: impl Serialize) {
        if let Ok(value) = serde_json::to_string(&value) {
            self.0.set_value(value);
        }
    }

    /// Returns the value of `self`.
    pub fn value_str(&self) -> &str {
        self.0.value()
    }

    /// Returns the value of `self` to the deserialized `value`.
    pub fn value<'de, T: Deserialize<'de>>(&'de self) -> Result<T, ParseCookieError> {
        serde_json::from_str(self.0.value()).map_err(ParseCookieError::ParseJsonValue)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Cookie {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let value = req
            .headers()
            .get(header::COOKIE)
            .ok_or(ParseCookieError::CookieHeaderRequired)?;
        let value = value
            .to_str()
            .map_err(|_| ParseCookieError::CookieIllegal)?;
        let cookie = libcookie::Cookie::parse_encoded(value.to_string())
            .map_err(|_| ParseCookieError::CookieIllegal)?;
        Ok(Cookie(cookie))
    }
}

/// A collection of cookies that tracks its modifications.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{header, StatusCode},
///     middleware::CookieJarManager,
///     test::TestClient,
///     web::cookie::{Cookie, CookieJar},
///     Endpoint, EndpointExt, Request, Route,
/// };
///
/// #[handler]
/// fn index(cookie_jar: &CookieJar) -> String {
///     let count = match cookie_jar.get("count") {
///         Some(cookie) => cookie.value::<i32>().unwrap() + 1,
///         None => 1,
///     };
///     cookie_jar.add(Cookie::new("count", count));
///     format!("count: {}", count)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new()
///     .at("/", get(index))
///     .with(CookieJarManager::new());
/// let cli = TestClient::new(app);
///
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
/// let cookie = resp.0.headers().get(header::SET_COOKIE).cloned().unwrap();
/// resp.assert_text("count: 1").await;
///
/// let resp = cli.get("/").header(header::COOKIE, cookie).send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("count: 2").await;
/// # });
/// ```
#[derive(Default, Clone)]
pub struct CookieJar {
    jar: Arc<Mutex<libcookie::CookieJar>>,
    pub(crate) key: Option<Arc<CookieKey>>,
}

impl CookieJar {
    /// Adds cookie to this jar. If a cookie with the same name already exists,
    /// it is replaced with cookie.
    pub fn add(&self, cookie: Cookie) {
        self.jar.lock().add(cookie.0);
    }

    /// Removes cookie from this jar.
    pub fn remove(&self, name: impl AsRef<str>) {
        self.jar
            .lock()
            .remove(libcookie::Cookie::named(name.as_ref().to_string()));
    }

    /// Returns a reference to the [`Cookie`] inside this jar with the `name`.
    /// If no such cookie exists, returns `None`.
    pub fn get(&self, name: &str) -> Option<Cookie> {
        self.jar.lock().get(name).cloned().map(Cookie)
    }

    /// Removes all delta cookies.
    pub fn reset_delta(&self) {
        self.jar.lock().reset_delta();
    }

    /// Wraps an iterator over all of the cookies present in this jar with a
    /// closure.
    pub fn with_cookies<F, R>(&self, f: F) -> R
    where
        F: FnOnce(libcookie::Iter) -> R,
    {
        let jar = self.jar.lock();
        let iter = jar.iter();
        f(iter)
    }

    /// Returns a PrivateJar with self as its parent jar using the key to
    /// encrypt and decrypt cookies added/retrieved from the child jar.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::web::cookie::{Cookie, CookieJar, CookieKey};
    ///
    /// let key = CookieKey::generate();
    /// let cookie_jar = CookieJar::default();
    ///
    /// cookie_jar
    ///     .private_with_key(&key)
    ///     .add(Cookie::new_with_str("foo", "bar"));
    ///
    /// assert_ne!(cookie_jar.get("foo").unwrap().value_str(), "bar");
    /// assert_eq!(
    ///     cookie_jar
    ///         .private_with_key(&key)
    ///         .get("foo")
    ///         .unwrap()
    ///         .value_str(),
    ///     "bar"
    /// );
    ///
    /// let key2 = CookieKey::generate();
    /// assert!(cookie_jar.private_with_key(&key2).get("foo").is_none());
    /// ```
    pub fn private_with_key<'a>(&'a self, key: &'a CookieKey) -> PrivateCookieJar<'a> {
        PrivateCookieJar {
            key,
            cookie_jar: self,
        }
    }

    /// Similar to the `private_with_key` function, but using the key specified
    /// by the `CookieJarManager::with_key`.
    pub fn private(&self) -> PrivateCookieJar {
        self.private_with_key(
            self.key
                .as_ref()
                .expect("You must use the `CookieJarManager::with_key` to specify a `CookieKey`."),
        )
    }

    /// Returns a SignedJar with self as its parent jar using the key
    /// key to verify cookies retrieved from the child jar. Any retrievals from
    /// the child jar will be made from the parent jar.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// use poem::web::cookie::{Cookie, CookieJar, CookieKey};
    ///
    /// let key = CookieKey::generate();
    /// let cookie_jar = CookieJar::default();
    ///
    /// cookie_jar
    ///     .signed_with_key(&key)
    ///     .add(Cookie::new_with_str("foo", "bar"));
    ///
    /// assert!(cookie_jar.get("foo").unwrap().value_str().contains("bar"));
    /// assert_eq!(
    ///     cookie_jar
    ///         .signed_with_key(&key)
    ///         .get("foo")
    ///         .unwrap()
    ///         .value_str(),
    ///     "bar"
    /// );
    ///
    /// let key2 = CookieKey::generate();
    /// assert!(cookie_jar.signed_with_key(&key2).get("foo").is_none());
    /// ```
    pub fn signed_with_key<'a>(&'a self, key: &'a CookieKey) -> SignedCookieJar<'a> {
        SignedCookieJar {
            key,
            cookie_jar: self,
        }
    }

    /// Similar to the `signed_with_key` function, but using the key specified
    /// by the `CookieJarManager::with_key`.
    pub fn signed(&self) -> SignedCookieJar {
        self.signed_with_key(
            self.key
                .as_ref()
                .expect("You must use the `CookieJarManager::with_key` to specify a `CookieKey`."),
        )
    }
}

impl FromStr for CookieJar {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cookie_jar = libcookie::CookieJar::new();

        for cookie_str in s.split(';').map(str::trim) {
            if let Ok(cookie) = libcookie::Cookie::parse_encoded(cookie_str) {
                cookie_jar.add_original(cookie.into_owned());
            }
        }

        Ok(CookieJar {
            jar: Arc::new(Mutex::new(cookie_jar)),
            key: None,
        })
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a CookieJar {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req.cookie())
    }
}

impl CookieJar {
    pub(crate) fn extract_from_headers(headers: &HeaderMap) -> Self {
        let mut cookie_jar = libcookie::CookieJar::new();

        for value in headers.get_all(header::COOKIE) {
            if let Ok(value) = value.to_str() {
                for cookie_str in value.split(';').map(str::trim) {
                    if let Ok(cookie) = libcookie::Cookie::parse_encoded(cookie_str) {
                        cookie_jar.add_original(cookie.into_owned());
                    }
                }
            }
        }

        CookieJar {
            jar: Arc::new(Mutex::new(cookie_jar)),
            key: None,
        }
    }

    pub(crate) fn append_delta_to_headers(&self, headers: &mut HeaderMap) {
        let cookie = self.jar.lock();
        for cookie in cookie.delta() {
            let value = cookie.encoded().to_string();
            if let Ok(value) = HeaderValue::from_str(&value) {
                headers.append(header::SET_COOKIE, value);
            }
        }
    }
}

/// A cryptographic master key for use with Signed and/or Private jars.
pub type CookieKey = libcookie::Key;

/// A child cookie jar that provides authenticated encryption for its cookies.
pub struct PrivateCookieJar<'a> {
    key: &'a CookieKey,
    cookie_jar: &'a CookieJar,
}

impl<'a> PrivateCookieJar<'a> {
    /// Adds cookie to the parent jar. The cookie’s value is encrypted with
    /// authenticated encryption assuring confidentiality, integrity, and
    /// authenticity.
    pub fn add(&self, cookie: Cookie) {
        let mut cookie_jar = self.cookie_jar.jar.lock();
        let mut private_cookie_jar = cookie_jar.private_mut(self.key);
        private_cookie_jar.add(cookie.0);
    }

    /// Removes cookie from the parent jar.
    pub fn remove(&self, name: impl AsRef<str>) {
        let mut cookie_jar = self.cookie_jar.jar.lock();
        let mut private_cookie_jar = cookie_jar.private_mut(self.key);
        private_cookie_jar.remove(libcookie::Cookie::named(name.as_ref().to_string()));
    }

    /// Returns cookie inside this jar with the name and authenticates and
    /// decrypts the cookie’s value, returning a Cookie with the decrypted
    /// value. If the cookie cannot be found, or the cookie fails to
    /// authenticate or decrypt, None is returned.
    pub fn get(&self, name: &str) -> Option<Cookie> {
        let cookie_jar = self.cookie_jar.jar.lock();
        let private_cookie_jar = cookie_jar.private(self.key);
        private_cookie_jar.get(name).map(Cookie)
    }
}

/// A child cookie jar that authenticates its cookies.
pub struct SignedCookieJar<'a> {
    key: &'a CookieKey,
    cookie_jar: &'a CookieJar,
}

impl<'a> SignedCookieJar<'a> {
    /// Adds cookie to the parent jar. The cookie’s value is signed assuring
    /// integrity and authenticity.
    pub fn add(&self, cookie: Cookie) {
        let mut cookie_jar = self.cookie_jar.jar.lock();
        let mut signed_cookie_jar = cookie_jar.signed_mut(self.key);
        signed_cookie_jar.add(cookie.0);
    }

    /// Removes cookie from the parent jar.
    pub fn remove(&self, name: impl AsRef<str>) {
        let mut cookie_jar = self.cookie_jar.jar.lock();
        let mut signed_cookie_jar = cookie_jar.signed_mut(self.key);
        signed_cookie_jar.remove(libcookie::Cookie::named(name.as_ref().to_string()));
    }

    /// Returns cookie inside this jar with the name and authenticates and
    /// decrypts the cookie’s value, returning a Cookie with the decrypted
    /// value. If the cookie cannot be found, or the cookie fails to
    /// authenticate or decrypt, None is returned.
    pub fn get(&self, name: &str) -> Option<Cookie> {
        let cookie_jar = self.cookie_jar.jar.lock();
        let signed_cookie_jar = cookie_jar.signed(self.key);
        signed_cookie_jar.get(name).map(Cookie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_jar() {
        let a = Cookie::new_with_str("a", 100.to_string());
        let b = Cookie::new_with_str("b", 200.to_string());
        let c = Cookie::new_with_str("c", 300.to_string());

        let cookie_str = format!("{a}; {b}");

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
            cookie_jar.remove("a");

            let mut headers = HeaderMap::new();
            cookie_jar.append_delta_to_headers(&mut headers);

            let mut values = headers.get_all(header::SET_COOKIE).into_iter();
            let value = values.next().unwrap();
            let remove_c = Cookie::parse(value.to_str().unwrap()).unwrap();
            assert_eq!(remove_c.name(), "a");
            assert_eq!(remove_c.value_str(), "");

            assert!(values.next().is_none());
        }
    }

    #[tokio::test]
    async fn test_cookie_extractor() {
        let req = Request::builder()
            .header(header::COOKIE, Cookie::new_with_str("a", "1").to_string())
            .finish();
        let (req, mut body) = req.split();
        let cookie = Cookie::from_request(&req, &mut body).await.unwrap();
        assert_eq!(cookie.name(), "a");
        assert_eq!(cookie.value_str(), "1");
    }

    #[tokio::test]
    async fn private() {
        let key = CookieKey::generate();
        let cookie_jar = CookieJar::default();
        let private = cookie_jar.private_with_key(&key);
        private.add(Cookie::new_with_str("a", "123"));

        assert_eq!(private.get("a").unwrap().value_str(), "123");
        assert!(!cookie_jar.get("a").unwrap().value_str().contains("123"));

        let new_key = CookieKey::generate();
        let private = cookie_jar.private_with_key(&new_key);
        assert_eq!(private.get("a"), None);
    }

    #[tokio::test]
    async fn signed() {
        let key = CookieKey::generate();
        let cookie_jar = CookieJar::default();
        let signed = cookie_jar.signed_with_key(&key);
        signed.add(Cookie::new_with_str("a", "123"));

        assert_eq!(signed.get("a").unwrap().value_str(), "123");
        assert!(cookie_jar.get("a").unwrap().value_str().contains("123"));

        let new_key = CookieKey::generate();
        let signed = cookie_jar.signed_with_key(&new_key);
        assert_eq!(signed.get("a"), None);
    }

    #[test]
    fn test_extract_from_multiple_cookie_headers() {
        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, HeaderValue::from_static("a=1"));
        headers.append(header::COOKIE, HeaderValue::from_static("b=2; c=3"));
        let cookie_jar = CookieJar::extract_from_headers(&headers);
        assert_eq!(cookie_jar.get("a").unwrap().value_str(), "1");
        assert_eq!(cookie_jar.get("b").unwrap().value_str(), "2");
        assert_eq!(cookie_jar.get("c").unwrap().value_str(), "3");
    }

    #[test]
    fn with_cookies() {
        let key = CookieKey::generate();
        let cookie_jar = CookieJar::default();
        let signed = cookie_jar.signed_with_key(&key);
        let private = cookie_jar.private_with_key(&key);

        cookie_jar.add(Cookie::new_with_str("a", "123"));
        signed.add(Cookie::new_with_str("b", "456"));
        private.add(Cookie::new_with_str("c", "789"));

        cookie_jar.with_cookies(|cookies| assert_eq!(cookies.count(), 3));

        let mut cookie_names = cookie_jar
            .with_cookies(|cookies| cookies.map(|c| c.name().to_string()).collect::<Vec<_>>());
        cookie_names.sort();

        assert_eq!(
            cookie_names,
            vec![String::from("a"), String::from("b"), String::from("c")]
        );
    }
}
