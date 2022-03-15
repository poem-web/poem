use std::time::Duration;

use crate::web::cookie::{Cookie, CookieJar, CookieKey, SameSite};

/// Cookie security for session.
pub enum CookieSecurity {
    /// Use the raw cookie value.
    ///
    /// **NOTE: It is not recommended to be used in a production environment.**
    Plain,

    /// Use the key to encrypt the cookie value.
    Private(CookieKey),

    /// Sign the cookie value with the key.
    Signed(CookieKey),
}

/// Cookie configuration for session.
pub struct CookieConfig {
    security: CookieSecurity,
    name: String,
    path: String,
    domain: Option<String>,
    secure: bool,
    http_only: bool,
    max_age: Option<Duration>,
    same_site: Option<SameSite>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            security: CookieSecurity::Plain,
            name: "poem-session".to_string(),
            path: "/".to_string(),
            domain: None,
            secure: true,
            http_only: true,
            max_age: None,
            same_site: None,
        }
    }
}

impl CookieConfig {
    /// Create a new `plain` CookieSession.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new `private` CookieSession.
    pub fn private(key: CookieKey) -> Self {
        Self {
            security: CookieSecurity::Private(key),
            ..Default::default()
        }
    }

    /// Create a new `signed` CookieSession.
    pub fn signed(key: CookieKey) -> Self {
        Self {
            security: CookieSecurity::Signed(key),
            ..Default::default()
        }
    }

    /// Sets the `name` to the session cookie.
    #[must_use]
    pub fn name(self, value: impl Into<String>) -> Self {
        Self {
            name: value.into(),
            ..self
        }
    }

    /// Sets the `Path` to the session cookie. Default is `/`.
    #[must_use]
    pub fn path(self, value: impl Into<String>) -> Self {
        Self {
            path: value.into(),
            ..self
        }
    }

    /// Sets the `Domain` to the session cookie.
    #[must_use]
    pub fn domain(self, value: impl Into<String>) -> Self {
        Self {
            domain: Some(value.into()),
            ..self
        }
    }

    /// Sets the `Secure` to the session cookie. Default is `true`.
    #[must_use]
    pub fn secure(self, value: bool) -> Self {
        Self {
            secure: value,
            ..self
        }
    }

    /// Sets the `HttpOnly` to the session cookie. Default is `true`.
    #[must_use]
    pub fn http_only(self, value: bool) -> Self {
        Self {
            http_only: value,
            ..self
        }
    }

    /// Sets the `SameSite` to the session cookie.
    #[must_use]
    pub fn same_site(self, value: impl Into<Option<SameSite>>) -> Self {
        Self {
            same_site: value.into(),
            ..self
        }
    }

    /// Sets the `MaxAge` to the session cookie.
    #[must_use]
    pub fn max_age(self, value: impl Into<Option<Duration>>) -> Self {
        Self {
            max_age: value.into(),
            ..self
        }
    }

    /// Returns the TTL(time-to-live) of the cookie.
    #[inline]
    pub(crate) fn ttl(&self) -> Option<Duration> {
        self.max_age
    }

    /// Set the cookie value to `CookieJar`.
    pub fn set_cookie_value(&self, cookie_jar: &CookieJar, value: &str) {
        let mut cookie = Cookie::new_with_str(&self.name, value);

        cookie.set_path(&self.path);

        if let Some(domain) = &self.domain {
            cookie.set_domain(domain);
        }

        cookie.set_secure(self.secure);
        cookie.set_http_only(self.http_only);

        if let Some(max_age) = &self.max_age {
            cookie.set_max_age(*max_age);
        }

        cookie.set_same_site(self.same_site);

        match &self.security {
            CookieSecurity::Plain => cookie_jar.add(cookie),
            CookieSecurity::Private(key) => cookie_jar.private_with_key(key).add(cookie),
            CookieSecurity::Signed(key) => cookie_jar.signed_with_key(key).add(cookie),
        }
    }

    /// Remove the cookie from `CookieJar`.
    pub fn remove_cookie(&self, cookie_jar: &CookieJar) {
        match &self.security {
            CookieSecurity::Plain => cookie_jar.remove(&self.name),
            CookieSecurity::Private(key) => cookie_jar.private_with_key(key).remove(&self.name),
            CookieSecurity::Signed(key) => cookie_jar.signed_with_key(key).remove(&self.name),
        }
    }

    /// Gets the cookie value from `CookieJar`.
    pub fn get_cookie_value(&self, cookie_jar: &CookieJar) -> Option<String> {
        let cookie = match &self.security {
            CookieSecurity::Plain => cookie_jar.get(&self.name),
            CookieSecurity::Private(key) => cookie_jar.private_with_key(key).get(&self.name),
            CookieSecurity::Signed(key) => cookie_jar.signed_with_key(key).get(&self.name),
        };
        cookie.map(|cookie| cookie.value_str().to_string())
    }
}
