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
/// use poem::{endpoint::make_sync, handler, http::header, Endpoint, Request, RouteDomain};
///
/// let app = RouteDomain::new()
///     .at("example.com", make_sync(|_| "1"))
///     .at("www.+.com", make_sync(|_| "2"))
///     .at("*.example.com", make_sync(|_| "3"))
///     .at("*", make_sync(|_| "4"));
///
/// fn make_request(host: &str) -> Request {
///     Request::builder().header(header::HOST, host).finish()
/// }
///
/// async fn do_request(app: &RouteDomain, req: Request) -> String {
///     app.call(req)
///         .await
///         .unwrap()
///         .into_body()
///         .into_string()
///         .await
///         .unwrap()
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// assert_eq!(do_request(&app, make_request("example.com")).await, "1");
/// assert_eq!(do_request(&app, make_request("www.abc.com")).await, "2");
/// assert_eq!(do_request(&app, make_request("a.b.example.com")).await, "3");
/// assert_eq!(do_request(&app, make_request("rust-lang.org")).await, "4");
/// assert_eq!(do_request(&app, Request::default()).await, "4");
/// # });
/// ```
#[derive(Default)]
pub struct RouteDomain {
    tree: Trie<BoxEndpoint<'static, Response>>,
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

#[async_trait::async_trait]
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
    use super::*;
    use crate::{endpoint::make_sync, handler, http::HeaderMap};

    async fn check(r: &RouteDomain, host: &str, value: &str) {
        let mut req = Request::builder();
        if !host.is_empty() {
            req = req.header(header::HOST, host);
        }
        assert_eq!(
            r.call(req.finish())
                .await
                .unwrap()
                .into_body()
                .into_string()
                .await
                .unwrap(),
            value
        );
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

        assert!(r
            .call(
                Request::builder()
                    .header(header::HOST, "rust-lang.org")
                    .finish()
            )
            .await
            .unwrap_err()
            .is::<NotFoundError>());

        assert!(r
            .call(Request::default())
            .await
            .unwrap_err()
            .is::<NotFoundError>());
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
