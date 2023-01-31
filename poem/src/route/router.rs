use std::{str::FromStr, sync::Arc};

use regex::Regex;

use crate::{
    endpoint::BoxEndpoint,
    error::{NotFoundError, RouteError},
    http::{uri::PathAndQuery, Uri},
    route::{check_result, internal::radix_tree::RadixTree},
    Endpoint, EndpointExt, IntoEndpoint, IntoResponse, Request, Response, Result,
};

#[derive(Debug)]
struct PathPrefix(usize);

/// Routing object
///
/// You can match the full path or wildcard path, and use the
/// [`Path`](crate::web::Path) extractor to get the path parameters.
///
/// # Errors
///
/// - [`NotFoundError`]
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     web::Path,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// async fn a() {}
///
/// #[handler]
/// async fn b(Path((group, name)): Path<(String, String)>) {
///     assert_eq!(group, "foo");
///     assert_eq!(name, "bar");
/// }
///
/// #[handler]
/// async fn c(Path(path): Path<String>) {
///     assert_eq!(path, "d/e");
/// }
///
/// let app = Route::new()
///     // full path
///     .at("/a/b", get(a))
///     // capture parameters
///     .at("/b/:group/:name", get(b))
///     // capture tail path
///     .at("/c/*path", get(c))
///     // match regex
///     .at("/d/<\\d+>", get(a))
///     // capture with regex
///     .at("/e/:name<\\d+>", get(a));
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let cli = TestClient::new(app);
///
/// // /a/b
/// cli.get("/a/b").send().await.assert_status_is_ok();
///
/// // /b/:group/:name
/// cli.get("/b/foo/bar").send().await.assert_status_is_ok();
///
/// // /c/*path
/// cli.get("/c/d/e").send().await.assert_status_is_ok();
///
/// // /d/<\\d>
/// cli.get("/d/123").send().await.assert_status_is_ok();
///
/// // /e/:name<\\d>
/// cli.get("/e/123").send().await.assert_status_is_ok();
/// # });
/// ```
///
/// # Nested
///
/// ```
/// use poem::{
///     handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// let app = Route::new().nest("/foo", Route::new().at("/bar", index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/foo/bar").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("hello").await;
/// # });
/// ```
///
/// # Nested no strip
///
/// ```
/// use poem::{
///     handler,
///     http::{StatusCode, Uri},
///     test::TestClient,
///     Endpoint, Request, Route,
/// };
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// let app = Route::new().nest_no_strip("/foo", Route::new().at("/foo/bar", index));
/// let cli = TestClient::new(app);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/foo/bar").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("hello").await;
/// # });
/// ```
#[derive(Default)]
pub struct Route {
    tree: RadixTree<BoxEndpoint<'static>>,
}

impl Route {
    /// Create a new routing object.
    pub fn new() -> Route {
        Default::default()
    }

    /// Add an [Endpoint] to the specified path.
    ///
    /// # Panics
    ///
    /// Panic when there are duplicates in the routing table.
    #[must_use]
    pub fn at<E>(self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        check_result(self.try_at(path, ep))
    }

    /// Attempts to add an [Endpoint] to the specified path.
    pub fn try_at<E>(mut self, path: impl AsRef<str>, ep: E) -> Result<Self, RouteError>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.tree
            .add(&normalize_path(path.as_ref()), ep.map_to_response().boxed())?;
        Ok(self)
    }

    /// Nest a `Endpoint` to the specified path and strip the prefix.
    ///
    /// # Panics
    ///
    /// Panic when there are duplicates in the routing table.
    #[must_use]
    pub fn nest<E>(self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        check_result(self.try_nest(path, ep))
    }

    /// Attempts to nest a `Endpoint` to the specified path and strip the
    /// prefix.
    pub fn try_nest<E>(self, path: impl AsRef<str>, ep: E) -> Result<Self, RouteError>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.internal_nest(&normalize_path(path.as_ref()), ep, true)
    }

    /// Nest a `Endpoint` to the specified path, but do not strip the prefix.
    ///
    /// # Panics
    ///
    /// Panic when there are duplicates in the routing table.
    #[must_use]
    pub fn nest_no_strip<E>(self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        check_result(self.try_nest_no_strip(path, ep))
    }

    /// Attempts to nest a `Endpoint` to the specified path, but do not strip
    /// the prefix.
    pub fn try_nest_no_strip<E>(self, path: impl AsRef<str>, ep: E) -> Result<Self, RouteError>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.internal_nest(&normalize_path(path.as_ref()), ep, false)
    }

    fn internal_nest<E>(mut self, path: &str, ep: E, strip: bool) -> Result<Self, RouteError>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        let ep = Arc::new(ep.into_endpoint());
        let mut path = path.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }

        struct Nest<T> {
            inner: T,
            root: bool,
            prefix_len: usize,
            prefix_for_path_pattern: usize,
        }

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> Result<Self::Output> {
                if !self.root {
                    let idx = req.state().match_params.len() - 1;
                    let (name, _) = req.state_mut().match_params.remove(idx);
                    assert_eq!(name, "--poem-rest");
                }

                let new_uri = {
                    let uri = std::mem::take(req.uri_mut());
                    let mut uri_parts = uri.into_parts();
                    let path =
                        &uri_parts.path_and_query.as_ref().unwrap().as_str()[self.prefix_len..];
                    uri_parts.path_and_query = Some(if !path.starts_with('/') {
                        PathAndQuery::from_str(&format!("/{path}")).unwrap()
                    } else {
                        PathAndQuery::from_str(path).unwrap()
                    });
                    Uri::from_parts(uri_parts).unwrap()
                };
                *req.uri_mut() = new_uri;

                req.set_data(PathPrefix(self.prefix_for_path_pattern));
                Ok(self.inner.call(req).await?.into_response())
            }
        }

        assert!(
            path.find('*').is_none(),
            "wildcards are not allowed in the nest path."
        );
        assert!(
            path.find(':').is_none(),
            "captures are not allowed in the nest path."
        );
        assert!(
            path.find('<').is_none(),
            "regexs are not allowed in the nest path."
        );

        let prefix_len = match strip {
            false => 0,
            true => path.len() - 1,
        };
        let prefix_for_path_pattern = match strip {
            false => path.len() - 1,
            true => 0,
        };

        self.tree.add(
            &format!("{path}*--poem-rest"),
            Box::new(Nest {
                inner: ep.clone(),
                root: false,
                prefix_len,
                prefix_for_path_pattern,
            }),
        )?;

        self.tree.add(
            &path[..path.len() - 1],
            Box::new(Nest {
                inner: ep,
                root: true,
                prefix_len,
                prefix_for_path_pattern,
            }),
        )?;

        Ok(self)
    }
}

#[derive(Debug, Clone)]
#[allow(unreachable_pub)]
pub struct PathPattern(pub Arc<str>);

#[async_trait::async_trait]
impl Endpoint for Route {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        match self.tree.matches(req.uri().path()) {
            Some(matches) => {
                req.state_mut().match_params.extend(matches.params);

                let pattern = match matches.data.pattern.strip_suffix("/*--poem-rest") {
                    Some(pattern) => pattern.into(),
                    None => matches.data.pattern.clone(),
                };

                let pattern = match (req.data::<PathPattern>(), req.data::<PathPrefix>()) {
                    (Some(parent), Some(prefix)) => {
                        PathPattern(format!("{}{}", parent.0, &pattern[prefix.0..]).into())
                    }
                    (None, Some(prefix)) => PathPattern(pattern[prefix.0..].into()),
                    (None, None) => PathPattern(pattern),
                    (Some(parent), None) => PathPattern(format!("{}{}", parent.0, pattern).into()),
                };
                req.set_data(pattern.clone());

                let result = matches.data.data.call(req).await;

                // Add PathPattern to the innermost response so that metrics instrumentation
                // can report the innermost matched pattern.
                match result {
                    Ok(mut res) => {
                        if res.data::<PathPattern>().is_none() {
                            res.set_data(pattern);
                        }
                        Ok(res)
                    }
                    Err(mut err) => {
                        if err.data::<PathPattern>().is_none() {
                            err.set_data(pattern);
                        }
                        Err(err)
                    }
                }
            }
            None => Err(NotFoundError.into()),
        }
    }
}

fn normalize_path(path: &str) -> String {
    let re = Regex::new("//+").unwrap();
    let mut path = re.replace_all(path, "/").to_string();
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    path
}

#[cfg(test)]
mod tests {
    use futures_util::lock::Mutex;
    use http::{StatusCode, Uri};

    use super::*;
    use crate::{endpoint::make_sync, handler, test::TestClient, Error};

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/a/b/c"), "/a/b/c");
        assert_eq!(normalize_path("/a///b//c"), "/a/b/c");
        assert_eq!(normalize_path("a/b/c"), "/a/b/c");
    }

    #[handler(internal)]
    fn h(uri: &Uri) -> String {
        uri.path().to_string()
    }

    async fn get(route: &impl Endpoint<Output = Response>, path: &'static str) -> String {
        route
            .call(Request::builder().uri(Uri::from_static(path)).finish())
            .await
            .unwrap()
            .take_body()
            .into_string()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn nested() {
        let r = Route::new().nest(
            "/",
            Route::new()
                .at("/a", h)
                .at("/b", h)
                .nest("/inner", Route::new().at("/c", h)),
        );

        assert_eq!(get(&r, "/a").await, "/a");
        assert_eq!(get(&r, "/b").await, "/b");
        assert_eq!(get(&r, "/inner/c").await, "/c");

        let r = Route::new().nest(
            "/api",
            Route::new()
                .at("/a", h)
                .at("/b", h)
                .nest("/inner", Route::new().at("/c", h)),
        );

        assert_eq!(get(&r, "/api/a").await, "/a");
        assert_eq!(get(&r, "/api/b").await, "/b");
        assert_eq!(get(&r, "/api/inner/c").await, "/c");
    }

    #[tokio::test]
    async fn nested_no_strip() {
        let r = Route::new().nest_no_strip(
            "/",
            Route::new()
                .at("/a", h)
                .at("/b", h)
                .nest_no_strip("/inner", Route::new().at("/inner/c", h)),
        );

        assert_eq!(get(&r, "/a").await, "/a");
        assert_eq!(get(&r, "/b").await, "/b");
        assert_eq!(get(&r, "/inner/c").await, "/inner/c");

        let r = Route::new().nest_no_strip(
            "/api",
            Route::new()
                .at("/api/a", h)
                .at("/api/b", h)
                .nest_no_strip("/api/inner", Route::new().at("/api/inner/c", h)),
        );

        assert_eq!(get(&r, "/api/a").await, "/api/a");
        assert_eq!(get(&r, "/api/b").await, "/api/b");
        assert_eq!(get(&r, "/api/inner/c").await, "/api/inner/c");
    }

    #[tokio::test]
    async fn nested_query_string() {
        let r = Route::new().nest(
            "/a",
            Route::new().nest(
                "/b",
                Route::new().at(
                    "/c",
                    make_sync(|req| req.uri().path_and_query().unwrap().to_string()),
                ),
            ),
        );
        assert_eq!(get(&r, "/a/b/c?name=abc").await, "/c?name=abc");
    }

    #[tokio::test]
    async fn nested2() {
        let r = Route::new().nest(
            "/a",
            Route::new().nest(
                "/",
                make_sync(|req| req.uri().path_and_query().unwrap().to_string()),
            ),
        );
        assert_eq!(get(&r, "/a").await, "/");
        assert_eq!(get(&r, "/a?a=1").await, "/?a=1");
    }

    #[test]
    #[should_panic]
    fn duplicate_1() {
        let _ = Route::new().at("/", h).at("/", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_2() {
        let _ = Route::new().at("/a", h).nest("/a", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_3() {
        let _ = Route::new().nest("/a", h).nest("/a", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_4() {
        let _ = Route::new().at("/a/:a", h).at("/a/:a", h);
    }

    #[test]
    #[should_panic]
    fn duplicate_5() {
        let _ = Route::new().at("/a/*:v", h).at("/a/*", h);
    }

    #[tokio::test]
    async fn issue_174() {
        let app = Route::new().nest("/", make_sync(|_| "hello"));
        assert_eq!(
            app.get_response(Request::builder().uri(Uri::from_static("a")).finish())
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn path_pattern() {
        let app = Route::new()
            .at(
                "/a/:id",
                make_sync(|req| req.data::<PathPattern>().unwrap().0.to_string()),
            )
            .nest(
                "/nest",
                Route::new().at(
                    "/c/:id",
                    make_sync(|req| req.data::<PathPattern>().unwrap().0.to_string()),
                ),
            )
            .nest(
                "/nest1",
                Route::new().nest(
                    "/nest2",
                    Route::new().at(
                        "/:id",
                        make_sync(|req| req.data::<PathPattern>().unwrap().0.to_string()),
                    ),
                ),
            )
            .nest_no_strip(
                "/nest_no_strip",
                Route::new().at(
                    "/nest_no_strip/d/:id",
                    make_sync(|req| req.data::<PathPattern>().unwrap().0.to_string()),
                ),
            )
            .nest_no_strip(
                "/nest_no_strip1",
                Route::new().nest_no_strip(
                    "/nest_no_strip1/nest_no_strip2",
                    Route::new().at(
                        "/nest_no_strip1/nest_no_strip2/:id",
                        make_sync(|req| req.data::<PathPattern>().unwrap().0.to_string()),
                    ),
                ),
            );

        let cli = TestClient::new(app);

        cli.get("/a/10").send().await.assert_text("/a/:id").await;
        cli.get("/nest/c/10")
            .send()
            .await
            .assert_text("/nest/c/:id")
            .await;
        cli.get("/nest_no_strip/d/99")
            .send()
            .await
            .assert_text("/nest_no_strip/d/:id")
            .await;
        cli.get("/nest1/nest2/10")
            .send()
            .await
            .assert_text("/nest1/nest2/:id")
            .await;
        cli.get("/nest_no_strip1/nest_no_strip2/10")
            .send()
            .await
            .assert_text("/nest_no_strip1/nest_no_strip2/:id")
            .await;
    }

    #[derive(Clone, Default)]
    struct PathPatternSpy {
        pattern: Arc<Mutex<Option<PathPattern>>>,
    }

    impl PathPatternSpy {
        async fn last_pattern(&self) -> String {
            let guard = self.pattern.lock().await;
            guard
                .as_ref()
                .map(|pp| pp.0.to_string())
                .expect("some pattern")
        }

        async fn set_last_pattern(&self, pattern: Option<&PathPattern>) {
            let mut guard = self.pattern.lock().await;
            *guard = pattern.cloned();
        }
    }

    impl<E: Endpoint> crate::Middleware<E> for PathPatternSpy {
        type Output = PathPatternSpyEndpoint<E>;

        fn transform(&self, ep: E) -> Self::Output {
            PathPatternSpyEndpoint {
                spy: self.clone(),
                inner: ep,
            }
        }
    }

    struct PathPatternSpyEndpoint<E> {
        spy: PathPatternSpy,
        inner: E,
    }

    #[async_trait::async_trait]
    impl<E: Endpoint> Endpoint for PathPatternSpyEndpoint<E> {
        type Output = Response;

        async fn call(&self, req: Request) -> Result<Self::Output> {
            let result = self.inner.call(req).await.map(IntoResponse::into_response);

            match result {
                Ok(res) => {
                    self.spy.set_last_pattern(res.data::<PathPattern>()).await;
                    Ok(res)
                }
                Err(err) => {
                    self.spy.set_last_pattern(err.data::<PathPattern>()).await;
                    Err(err)
                }
            }
        }
    }

    #[tokio::test]
    async fn path_pattern_middleware_with_ok_result() {
        let spy = PathPatternSpy::default();
        let app = Route::new()
            .at("/a/:id", make_sync(|_| "ok"))
            .nest("/nest", Route::new().at("/c/:id", make_sync(|_| "ok")))
            .nest(
                "/nest1",
                Route::new().nest("/nest2", Route::new().at("/:id", make_sync(|_| "ok"))),
            )
            .nest_no_strip(
                "/nest_no_strip",
                Route::new().at("/nest_no_strip/d/:id", make_sync(|_| "ok")),
            )
            .nest_no_strip(
                "/nest_no_strip1",
                Route::new().nest_no_strip(
                    "/nest_no_strip1/nest_no_strip2",
                    Route::new().at("/nest_no_strip1/nest_no_strip2/:id", make_sync(|_| "ok")),
                ),
            )
            .with(spy.clone());

        let cli = TestClient::new(app);

        cli.get("/a/10").send().await.assert_status_is_ok();
        assert_eq!(spy.last_pattern().await, "/a/:id");
        cli.get("/nest/c/10").send().await.assert_status_is_ok();
        assert_eq!(spy.last_pattern().await, "/nest/c/:id");
        cli.get("/nest_no_strip/d/99")
            .send()
            .await
            .assert_status_is_ok();
        assert_eq!(spy.last_pattern().await, "/nest_no_strip/d/:id");
        cli.get("/nest1/nest2/10")
            .send()
            .await
            .assert_status_is_ok();
        assert_eq!(spy.last_pattern().await, "/nest1/nest2/:id");
        cli.get("/nest_no_strip1/nest_no_strip2/10")
            .send()
            .await
            .assert_status_is_ok();
        assert_eq!(
            spy.last_pattern().await,
            "/nest_no_strip1/nest_no_strip2/:id"
        );
    }

    struct ErrorEndpoint;

    #[async_trait::async_trait]
    impl Endpoint for ErrorEndpoint {
        type Output = Response;

        async fn call(&self, _req: Request) -> Result<Self::Output> {
            Err(Error::from_status(StatusCode::SERVICE_UNAVAILABLE))
        }
    }

    #[tokio::test]
    async fn path_pattern_middleware_with_err_result() {
        let spy = PathPatternSpy::default();
        let app = Route::new()
            .at("/a/:id", ErrorEndpoint)
            .nest("/nest", Route::new().at("/c/:id", ErrorEndpoint))
            .nest(
                "/nest1",
                Route::new().nest("/nest2", Route::new().at("/:id", ErrorEndpoint)),
            )
            .nest_no_strip(
                "/nest_no_strip",
                Route::new().at("/nest_no_strip/d/:id", ErrorEndpoint),
            )
            .nest_no_strip(
                "/nest_no_strip1",
                Route::new().nest_no_strip(
                    "/nest_no_strip1/nest_no_strip2",
                    Route::new().at("/nest_no_strip1/nest_no_strip2/:id", ErrorEndpoint),
                ),
            )
            .with(spy.clone());

        let cli = TestClient::new(app);

        cli.get("/a/10")
            .send()
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(spy.last_pattern().await, "/a/:id");
        cli.get("/nest/c/10")
            .send()
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(spy.last_pattern().await, "/nest/c/:id");
        cli.get("/nest_no_strip/d/99")
            .send()
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(spy.last_pattern().await, "/nest_no_strip/d/:id");
        cli.get("/nest1/nest2/10")
            .send()
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(spy.last_pattern().await, "/nest1/nest2/:id");
        cli.get("/nest_no_strip1/nest_no_strip2/10")
            .send()
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            spy.last_pattern().await,
            "/nest_no_strip1/nest_no_strip2/:id"
        );
    }
}
