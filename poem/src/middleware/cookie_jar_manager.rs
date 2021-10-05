use crate::{web::CookieJar, Endpoint, IntoResponse, Middleware, Request, Response};

/// Middleware for CookieJar support.
#[derive(Default)]
pub(crate) struct CookieJarManager;

impl<E> Middleware<E> for CookieJarManager
where
    E: Endpoint,
{
    type Output = CookieJarManagerEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManagerEndpoint { inner: ep }
    }
}

/// Endpoint for CookieJarManager middleware.
pub(crate) struct CookieJarManagerEndpoint<E> {
    inner: E,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CookieJarManagerEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        let cookie_jar = CookieJar::extract_from_headers(req.headers());
        if req.state().cookie_jar.is_none() {
            req.state_mut().cookie_jar = Some(cookie_jar.clone());
            let mut resp = self.inner.call(req).await.into_response();
            cookie_jar.append_delta_to_headers(resp.headers_mut());
            resp
        } else {
            self.inner.call(req).await.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, http::StatusCode, EndpointExt};

    #[tokio::test]
    async fn test_cookie_jar_manager() {
        #[handler(internal)]
        async fn index(cookie_jar: &CookieJar) {
            assert_eq!(cookie_jar.get("value").unwrap().value(), "88");
        }

        let ep = index.with(CookieJarManager);
        let resp = ep
            .call(Request::builder().header("Cookie", "value=88").finish())
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
