use std::{str::FromStr, sync::Arc};

use http::StatusCode;
use regex::Regex;

use super::tree::Tree;
use crate::{
    http::{uri::PathAndQuery, Method, Uri},
    Body, Endpoint, EndpointExt, IntoEndpoint, IntoResponse, Request, Response,
};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Tree<Box<dyn Endpoint<Output = Response>>>,
}

impl Route {
    /// Create a new routing object.
    pub fn new() -> Route {
        Default::default()
    }

    /// Add an [Endpoint] to the specified path.
    ///
    /// You can match the full path or wildcard path, and use the
    /// [`Path`](crate::web::Path) extractor to get the path parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     get, handler,
    ///     http::{StatusCode, Uri},
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
    /// // /a/b
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/a/b")).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    ///
    /// // /b/:group/:name
    /// let resp = app
    ///     .call(
    ///         Request::builder()
    ///             .uri(Uri::from_static("/b/foo/bar"))
    ///             .finish(),
    ///     )
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    ///
    /// // /c/*path
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/c/d/e")).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    ///
    /// // /d/<\\d>
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/d/123")).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    ///
    /// // /e/:name<\\d>
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/e/123")).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// # });
    /// ```
    #[must_use]
    pub fn at<E>(mut self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.router.add(
            &normalize_path(path.as_ref()),
            Box::new(ep.into_endpoint().map_to_response()),
        );
        self
    }

    /// Nest a `Endpoint` to the specified path and strip the prefix.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     handler,
    ///     http::{StatusCode, Uri},
    ///     Endpoint, Request, Route,
    /// };
    ///
    /// #[handler]
    /// fn index() -> &'static str {
    ///     "hello"
    /// }
    ///
    /// let app = Route::new().nest("/foo", Route::new().at("/bar", index));
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app
    ///     .call(
    ///         Request::builder()
    ///             .uri(Uri::from_static("/foo/bar"))
    ///             .finish(),
    ///     )
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
    /// # });
    /// ```
    #[must_use]
    pub fn nest<E>(self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.internal_nest(&normalize_path(path.as_ref()), ep, true)
    }

    /// Nest a `Endpoint` to the specified path, but do not strip the prefix.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     handler,
    ///     http::{StatusCode, Uri},
    ///     Endpoint, Request, Route,
    /// };
    ///
    /// #[handler]
    /// fn index() -> &'static str {
    ///     "hello"
    /// }
    ///
    /// let app = Route::new().nest_no_strip("/foo", Route::new().at("/foo/bar", index));
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app
    ///     .call(
    ///         Request::builder()
    ///             .uri(Uri::from_static("/foo/bar"))
    ///             .finish(),
    ///     )
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
    /// # });
    /// ```
    #[must_use]
    pub fn nest_no_strip<E>(self, path: impl AsRef<str>, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.internal_nest(&normalize_path(path.as_ref()), ep, false)
    }

    fn internal_nest<E>(mut self, path: &str, ep: E, strip: bool) -> Self
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
        }

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> Self::Output {
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
                        PathAndQuery::from_str(&format!("/{}", path)).unwrap()
                    } else {
                        PathAndQuery::from_str(path).unwrap()
                    });
                    Uri::from_parts(uri_parts).unwrap()
                };
                *req.uri_mut() = new_uri;
                self.inner.call(req).await.into_response()
            }
        }

        assert!(
            path.find('*').is_none(),
            "wildcards are not allowed in the nest path."
        );

        let prefix_len = match strip {
            false => 0,
            true => path.len() - 1,
        };
        self.router.add(
            &format!("{}*--poem-rest", path),
            Box::new(Nest {
                inner: ep.clone(),
                root: false,
                prefix_len,
            }),
        );
        self.router.add(
            &path[..path.len() - 1],
            Box::new(Nest {
                inner: ep,
                root: true,
                prefix_len,
            }),
        );

        self
    }
}

#[async_trait::async_trait]
impl Endpoint for Route {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        match self.router.matches(req.uri().path()) {
            Some(matches) => {
                req.state_mut().match_params.extend(matches.params);
                matches.data.call(req).await
            }
            None => StatusCode::NOT_FOUND.into(),
        }
    }
}

/// Routing object for HTTP methods
#[derive(Default)]
pub struct RouteMethod {
    methods: Vec<(Method, Box<dyn Endpoint<Output = Response>>)>,
}

impl RouteMethod {
    /// Create a `RouteMethod` object.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     handler,
    ///     http::{Method, StatusCode},
    ///     Endpoint, Request, RouteMethod,
    /// };
    ///
    /// #[handler]
    /// fn handle_get() -> &'static str {
    ///     "get"
    /// }
    ///
    /// #[handler]
    /// fn handle_post() -> &'static str {
    ///     "post"
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let route_method = RouteMethod::new().get(handle_get).post(handle_post);
    ///
    /// let resp = route_method
    ///     .call(Request::builder().method(Method::GET).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "get");
    ///
    /// let resp = route_method
    ///     .call(Request::builder().method(Method::POST).finish())
    ///     .await;
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "post");
    /// # });
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the endpoint for specified `method`.
    pub fn method<E>(mut self, method: Method, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.methods
            .push((method, Box::new(ep.into_endpoint().map_to_response())));
        self
    }

    /// Sets the endpoint for `GET`.
    pub fn get<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::GET, ep)
    }

    /// Sets the endpoint for `POST`.
    pub fn post<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::POST, ep)
    }

    /// Sets the endpoint for `PUT`.
    pub fn put<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::PUT, ep)
    }

    /// Sets the endpoint for `DELETE`.
    pub fn delete<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::DELETE, ep)
    }

    /// Sets the endpoint for `HEAD`.
    pub fn head<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::HEAD, ep)
    }

    /// Sets the endpoint for `OPTIONS`.
    pub fn options<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::OPTIONS, ep)
    }

    /// Sets the endpoint for `CONNECT`.
    pub fn connect<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::CONNECT, ep)
    }

    /// Sets the endpoint for `PATCH`.
    pub fn patch<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::PATCH, ep)
    }

    /// Sets the endpoint for `TRACE`.
    pub fn trace<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::TRACE, ep)
    }
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        match self
            .methods
            .iter()
            .find(|(method, _)| method == req.method())
            .map(|(_, ep)| ep)
        {
            Some(ep) => ep.call(req).await,
            None => {
                if req.method() == Method::HEAD {
                    req.set_method(Method::GET);
                    let mut resp = self.call(req).await;
                    resp.set_body(Body::empty());
                    return resp;
                }
                StatusCode::NOT_FOUND.into()
            }
        }
    }
}

/// A helper function, similar to `RouteMethod::new().get(ep)`.
pub fn get<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().get(ep)
}

/// A helper function, similar to `RouteMethod::new().post(ep)`.
pub fn post<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().post(ep)
}

/// A helper function, similar to `RouteMethod::new().put(ep)`.
pub fn put<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().put(ep)
}

/// A helper function, similar to `RouteMethod::new().delete(ep)`.
pub fn delete<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().delete(ep)
}

/// A helper function, similar to `RouteMethod::new().head(ep)`.
pub fn head<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().head(ep)
}

/// A helper function, similar to `RouteMethod::new().options(ep)`.
pub fn options<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().options(ep)
}

/// A helper function, similar to `RouteMethod::new().connect(ep)`.
pub fn connect<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().connect(ep)
}

/// A helper function, similar to `RouteMethod::new().patch(ep)`.
pub fn patch<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().patch(ep)
}

/// A helper function, similar to `RouteMethod::new().trace(ep)`.
pub fn trace<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().trace(ep)
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
    use http::Uri;

    use super::*;
    use crate::{endpoint::make_sync, handler};

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

    #[tokio::test]
    async fn route_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        for method in &[
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ] {
            let route = RouteMethod::new().method(method.clone(), index).post(index);
            let resp = route
                .call(Request::builder().method(method.clone()).finish())
                .await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
        }

        macro_rules! test_method {
            ($(($id:ident, $method:ident)),*) => {
                $(
                let route = RouteMethod::new().$id(index).post(index);
                let resp = route
                    .call(Request::builder().method(Method::$method).finish())
                    .await;
                assert_eq!(resp.status(), StatusCode::OK);
                assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
                )*
            };
        }

        test_method!(
            (get, GET),
            (post, POST),
            (delete, DELETE),
            (put, PUT),
            (head, HEAD),
            (options, OPTIONS),
            (connect, CONNECT),
            (patch, PATCH),
            (trace, TRACE)
        );
    }

    #[tokio::test]
    async fn head_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        let route = RouteMethod::new().get(index);
        let resp = route
            .call(Request::builder().method(Method::HEAD).finish())
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.into_body().into_vec().await.unwrap().is_empty());
    }
}
