use std::sync::Arc;

use crate::{
    web::cookie::{CookieJar, CookieKey},
    Endpoint, IntoResponse, Middleware, Request, Response, Result,
};

/// Middleware for CookieJar support.
#[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
#[derive(Default)]
pub struct CookieJarManager {
    key: Option<Arc<CookieKey>>,
}

impl CookieJarManager {
    /// Creates a new `Compression` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Specify the `CookieKey` used for the `CookieJar::private` and
    /// `CookieJar::signed` methods.
    pub fn with_key(key: CookieKey) -> Self {
        Self {
            key: Some(Arc::new(key)),
        }
    }
}

impl<E> Middleware<E> for CookieJarManager
where
    E: Endpoint,
{
    type Output = CookieJarManagerEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManagerEndpoint {
            inner: ep,
            key: self.key.clone(),
        }
    }
}

/// Endpoint for `CookieJarManager` middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
pub struct CookieJarManagerEndpoint<E> {
    inner: E,
    key: Option<Arc<CookieKey>>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CookieJarManagerEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        if req.state().cookie_jar.is_none() {
            let mut cookie_jar = CookieJar::extract_from_headers(req.headers());
            cookie_jar.key = self.key.clone();
            req.state_mut().cookie_jar = Some(cookie_jar.clone());
            let mut resp = self.inner.call(req).await?.into_response();
            cookie_jar.append_delta_to_headers(resp.headers_mut());
            Ok(resp)
        } else {
            self.inner.call(req).await.map(IntoResponse::into_response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, http::StatusCode, web::cookie::Cookie, EndpointExt};

    #[tokio::test]
    async fn test_cookie_jar_manager() {
        #[handler(internal)]
        async fn index(cookie_jar: &CookieJar) {
            assert_eq!(cookie_jar.get("value").unwrap().value_str(), "88");
        }

        let ep = index.with(CookieJarManager::new());
        let resp = ep
            .call(Request::builder().header("Cookie", "value=88").finish())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cookie_jar_manager_with_key() {
        #[handler(internal)]
        async fn index(cookie_jar: &CookieJar) {
            assert_eq!(
                cookie_jar.private().get("value1").unwrap().value_str(),
                "88"
            );
            assert_eq!(cookie_jar.signed().get("value2").unwrap().value_str(), "99");
        }

        let key = CookieKey::generate();

        let ep = index.with(CookieJarManager::with_key(key.clone()));
        let cookie_jar = CookieJar::default();
        cookie_jar
            .private_with_key(&key)
            .add(Cookie::new_with_str("value1", "88"));
        cookie_jar
            .signed_with_key(&key)
            .add(Cookie::new_with_str("value2", "99"));
        let resp = ep
            .call(
                Request::builder()
                    .header(
                        "Cookie",
                        &format!(
                            "value1={}; value2={}",
                            cookie_jar.get("value1").unwrap().value_str(),
                            cookie_jar.get("value2").unwrap().value_str()
                        ),
                    )
                    .finish(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
