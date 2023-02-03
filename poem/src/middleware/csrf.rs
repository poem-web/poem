use std::{sync::Arc, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use libcsrf::{
    AesGcmCsrfProtection, CsrfCookie as RawCsrfCookie, CsrfProtection, CsrfToken as RawCsrfToken,
    UnencryptedCsrfCookie,
};

use crate::{
    middleware::{CookieJarManager, CookieJarManagerEndpoint},
    web::{
        cookie::{Cookie, SameSite},
        CsrfToken, CsrfVerifier,
    },
    Endpoint, Middleware, Request, Result,
};

/// Middleware for Cross-Site Request Forgery (CSRF) protection.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{header, Method, StatusCode},
///     middleware::Csrf,
///     post,
///     test::TestClient,
///     web::{cookie::Cookie, CsrfToken, CsrfVerifier},
///     Endpoint, EndpointExt, Error, Request, Result, Route,
/// };
/// use serde::Deserialize;
///
/// #[handler]
/// async fn login_ui(token: &CsrfToken) -> String {
///     token.0.clone()
/// }
///
/// #[handler]
/// async fn login(verifier: &CsrfVerifier, req: &Request) -> Result<String> {
///     let csrf_token = req
///         .header("X-CSRF-Token")
///         .ok_or_else(|| Error::from_status(StatusCode::UNAUTHORIZED))?;
///
///     if !verifier.is_valid(&csrf_token) {
///         return Err(Error::from_status(StatusCode::UNAUTHORIZED));
///     }
///
///     Ok(format!("login success"))
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new()
///     .at("/", get(login_ui).post(login))
///     .with(Csrf::new());
/// let cli = TestClient::new(app);
///
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
///
/// let cookie = resp.0.headers().get(header::SET_COOKIE).unwrap();
/// let cookie = Cookie::parse(cookie.to_str().unwrap()).unwrap();
/// let csrf_token = resp.0.into_body().into_string().await.unwrap();
///
/// let resp = cli
///     .post("/")
///     .header("X-CSRF-Token", csrf_token)
///     .header(
///         header::COOKIE,
///         format!("{}={}", cookie.name(), cookie.value_str()),
///     )
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("login success").await;
/// # });
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
pub struct Csrf {
    cookie_name: String,
    key: [u8; 32],
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
    ttl: Duration,
}

impl Default for Csrf {
    fn default() -> Self {
        Self {
            cookie_name: "poem-csrf-token".to_string(),
            key: Default::default(),
            secure: true,
            http_only: true,
            same_site: Some(SameSite::Strict),
            ttl: Duration::from_secs(24 * 60 * 60),
        }
    }
}

impl Csrf {
    /// Create `Csrf` middleware.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets AES256 key to provide signed, encrypted CSRF tokens and cookies.
    #[must_use]
    pub fn key(self, key: [u8; 32]) -> Self {
        Self { key, ..self }
    }

    /// Sets the `Secure` to the csrf cookie. Default is `true`.
    #[must_use]
    pub fn secure(self, value: bool) -> Self {
        Self {
            secure: value,
            ..self
        }
    }

    /// Sets the `HttpOnly` to the csrf cookie. Default is `true`.
    #[must_use]
    pub fn http_only(self, value: bool) -> Self {
        Self {
            http_only: value,
            ..self
        }
    }

    /// Sets the `SameSite` to the csrf cookie. Default is
    /// [`SameSite::Strict`](libcookie::SameSite::Strict).
    #[must_use]
    pub fn same_site(self, value: impl Into<Option<SameSite>>) -> Self {
        Self {
            same_site: value.into(),
            ..self
        }
    }

    /// Sets the protection ttl. This will be used for both the cookie
    /// expiry and the time window over which CSRF tokens are considered
    /// valid.
    ///
    /// The default for this value is one day.
    #[must_use]
    pub fn ttl(self, ttl: Duration) -> Self {
        Self { ttl, ..self }
    }
}

impl<E: Endpoint> Middleware<E> for Csrf {
    type Output = CookieJarManagerEndpoint<CsrfEndpoint<E>>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManager::new().transform(CsrfEndpoint {
            inner: ep,
            protect: Arc::new(AesGcmCsrfProtection::from_key(self.key)),
            cookie_name: self.cookie_name.clone(),
            secure: self.secure,
            http_only: self.http_only,
            same_site: self.same_site,
            ttl: self.ttl,
        })
    }
}

/// Endpoint for Csrf middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "csrf")))]
pub struct CsrfEndpoint<E> {
    inner: E,
    protect: Arc<AesGcmCsrfProtection>,
    cookie_name: String,
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
    ttl: Duration,
}

impl<E> CsrfEndpoint<E> {
    fn generate_token(
        &self,
        existing_cookie: Option<&UnencryptedCsrfCookie>,
    ) -> (RawCsrfToken, RawCsrfCookie) {
        let existing_cookie_bytes = existing_cookie.and_then(|c| {
            let c = c.value();
            if c.len() < 64 {
                None
            } else {
                let mut buf = [0; 64];
                buf.copy_from_slice(c);
                Some(buf)
            }
        });

        self.protect
            .generate_token_pair(existing_cookie_bytes.as_ref(), self.ttl.as_secs() as i64)
            .expect("couldn't generate token/cookie pair")
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CsrfEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let existing_cookie = req
            .cookie()
            .get(&self.cookie_name)
            .and_then(|cookie| STANDARD.decode(cookie.value_str()).ok())
            .and_then(|value| self.protect.parse_cookie(&value).ok());

        let (token, cookie) = self.generate_token(existing_cookie.as_ref());
        let csrf_cookie = {
            let mut cookie =
                Cookie::new_with_str(&self.cookie_name, STANDARD.encode(cookie.value()));
            cookie.set_secure(self.secure);
            cookie.set_http_only(self.http_only);
            cookie.set_same_site(self.same_site);
            cookie.set_max_age(self.ttl);
            cookie
        };

        req.cookie().add(csrf_cookie);
        req.extensions_mut()
            .insert(CsrfToken(STANDARD.encode(token.value())));
        req.extensions_mut()
            .insert(CsrfVerifier::new(existing_cookie, self.protect.clone()));

        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use http::{header, Method, StatusCode};

    use super::*;
    use crate::{get, handler, EndpointExt, Error, IntoResponse, Result};

    const CSRF_TOKEN_NAME: &str = "X-CSRF-Token";

    #[tokio::test]
    async fn test_csrf() {
        #[handler(internal)]
        fn login_ui(token: &CsrfToken) -> impl IntoResponse {
            token.0.to_string()
        }

        #[handler(internal)]
        fn login(verifier: &CsrfVerifier, req: &Request) -> Result<impl IntoResponse> {
            let token = req
                .header(CSRF_TOKEN_NAME)
                .ok_or_else(|| Error::from_string("missing token", StatusCode::BAD_REQUEST))?;
            match verifier.is_valid(token) {
                true => Ok("ok"),
                false => Err(Error::from_string("invalid token", StatusCode::BAD_REQUEST)),
            }
        }

        let app = get(login_ui).post(login).with(Csrf::new());

        for _ in 0..5 {
            let resp = app.call(Request::default()).await.unwrap();
            let cookie = resp
                .header(header::SET_COOKIE)
                .map(|cookie| cookie.to_string())
                .unwrap();
            let token = resp.into_body().into_string().await.unwrap();

            let resp = app
                .call(
                    Request::builder()
                        .method(Method::POST)
                        .header(CSRF_TOKEN_NAME, token)
                        .header(header::COOKIE, cookie)
                        .finish(),
                )
                .await
                .unwrap()
                .into_body()
                .into_string()
                .await
                .unwrap();
            assert_eq!(resp, "ok");
        }

        let resp = app.call(Request::default()).await.unwrap();
        let cookie = resp
            .header(header::SET_COOKIE)
            .map(|cookie| cookie.to_string())
            .unwrap();
        let token = resp.into_body().into_string().await.unwrap();

        let mut token = STANDARD.decode(token).unwrap();
        token[0] = token[0].wrapping_add(1);

        assert_eq!(
            app.call(
                Request::builder()
                    .method(Method::POST)
                    .header(CSRF_TOKEN_NAME, STANDARD.encode(token))
                    .header(header::COOKIE, cookie)
                    .finish(),
            )
            .await
            .unwrap_err()
            .to_string(),
            "invalid token"
        );
    }
}
