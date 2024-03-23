use crate::{
    endpoint::BoxEndpoint,
    error::{NotFoundError, RouteError},
    http::header,
    route::{check_result, internal::trie::Trie},
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, Result,
};

/// Routing object for `HOST` header
///
/// # Errors
///
/// - [`NotFoundError`]
///
/// # Example
///
/// ```
/// use poem::{
///     endpoint::make_sync,
///     handler,
///     http::header,
///     test::{TestClient, TestRequestBuilder},
///     Endpoint, Request, RouteDomain,
/// };
///
/// let app = RouteDomain::new()
///     .at("example.com", make_sync(|_| "1"))
///     .at("www.+.com", make_sync(|_| "2"))
///     .at("*.example.com", make_sync(|_| "3"))
///     .at("*", make_sync(|_| "4"));
///
/// async fn check(app: impl Endpoint, domain: Option<&str>, res: &str) {
///     let cli = TestClient::new(app);
///     let mut req = cli.get("/");
///     if let Some(domain) = domain {
///         req = req.header(header::HOST, domain);
///     }
///     let resp = req.send().await;
///     resp.assert_status_is_ok();
///     resp.assert_text(res).await;
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// check(&app, Some("example.com"), "1").await;
/// check(&app, Some("www.abc.com"), "2").await;
/// check(&app, Some("a.b.example.com"), "3").await;
/// check(&app, Some("rust-lang.org"), "4").await;
/// check(&app, None, "4").await;
/// # });
/// ```
#[derive(Default)]
pub struct RouteDomain {
    tree: Trie<BoxEndpoint<'static>>,
}

impl RouteDomain {
    /// Create a `RouteDomain` object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Add an [Endpoint] to the specified domain pattern.
    ///
    /// # Panics
    ///
    /// Panic when there are duplicates in the routing table.
    #[must_use]
    pub fn at<E>(self, pattern: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        check_result(self.try_at(pattern, ep))
    }

    /// Attempts to add an [Endpoint] to the specified domain pattern.
    pub fn try_at<E>(mut self, pattern: impl AsRef<str>, ep: E) -> Result<Self, RouteError>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.tree.add(
            pattern.as_ref(),
            ep.into_endpoint().map_to_response().boxed(),
        )?;
        Ok(self)
    }
}

impl Endpoint for RouteDomain {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|host| host.to_str().ok())
            .unwrap_or_default();
        match self.tree.matches(host) {
            Some(ep) => ep.call(req).await,
            None => Err(NotFoundError.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{endpoint::make_sync, handler, http::HeaderMap, test::TestClient};

    async fn check(r: &RouteDomain, host: &str, value: &str) {
        let cli = TestClient::new(r);
        let mut req = cli.get("/");
        if !host.is_empty() {
            req = req.header(header::HOST, host);
        }
        let resp = req.send().await;
        resp.assert_status_is_ok();
        resp.assert_text(value).await;
    }

    #[tokio::test]
    async fn route_domain() {
        #[handler(internal)]
        fn h(headers: &HeaderMap) -> String {
            headers
                .get(header::HOST)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        }

        let r = RouteDomain::new()
            .at("example.com", make_sync(|_| "1"))
            .at("www.example.com", make_sync(|_| "2"))
            .at("www.+.com", make_sync(|_| "3"))
            .at("*.com", make_sync(|_| "4"))
            .at("*", make_sync(|_| "5"));

        check(&r, "example.com", "1").await;
        check(&r, "www.example.com", "2").await;
        check(&r, "www.abc.com", "3").await;
        check(&r, "abc.com", "4").await;
        check(&r, "rust-lang.org", "5").await;
        check(&r, "", "5").await;
    }

    #[tokio::test]
    async fn not_found() {
        let r = RouteDomain::new()
            .at("example.com", make_sync(|_| "1"))
            .at("www.example.com", make_sync(|_| "2"))
            .at("www.+.com", make_sync(|_| "3"))
            .at("*.com", make_sync(|_| "4"));
        let cli = TestClient::new(r);

        cli.get("/")
            .header(header::HOST, "rust-lang.org")
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);

        cli.get("/")
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[handler(internal)]
    fn h() {}

    #[test]
    #[should_panic]
    fn duplicate_1() {
        let _ = RouteDomain::new().at("example.com", h).at("example.com", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_2() {
        let _ = RouteDomain::new()
            .at("+.example.com", h)
            .at("+.example.com", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_3() {
        let _ = RouteDomain::new()
            .at("*.example.com", h)
            .at("*.example.com", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_4() {
        let _ = RouteDomain::new().at("*", h).at("*", h);
    }
}
