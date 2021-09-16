//! Route object and DSL

use std::sync::Arc;

use fnv::FnvHashMap;
use http::StatusCode;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    http::Method, route_recognizer::Router, Body, Endpoint, EndpointExt, IntoEndpoint,
    IntoResponse, Request, Response,
};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Router<Box<dyn Endpoint<Output = Response>>>,
}

impl Route {
    /// Add an [Endpoint] to the specified path.
    ///
    /// You can match the full path or wildcard path, and use the
    /// [`Path`](crate::web::Path) extractor to get the path parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, route, route::get, web::Path};
    ///
    /// #[handler]
    /// async fn a() {}
    ///
    /// #[handler]
    /// async fn b(Path((group, name)): Path<(String, String)>) {}
    ///
    /// #[handler]
    /// async fn c(Path(path): Path<String>) {}
    ///
    /// let mut app = route()
    ///     // full path
    ///     .at("/a/b", get(a))
    ///     // capture parameters
    ///     .at("/b/:group/:name", get(b))
    ///     // capture tail path
    ///     .at("/c/*path", get(c));
    /// ```
    #[must_use]
    pub fn at(mut self, path: impl AsRef<str>, ep: impl IntoEndpoint) -> Self {
        self.router.add(
            &normalize_path(path.as_ref()),
            Box::new(ep.into_endpoint().map_to_response()),
        );
        self
    }

    /// Nest a `Endpoint` to the specified path and strip the prefix.
    #[must_use]
    pub fn nest(self, path: impl AsRef<str>, ep: impl IntoEndpoint) -> Self {
        self.internal_nest(&normalize_path(path.as_ref()), ep, true)
    }

    /// Nest a `Endpoint` to the specified path, but do not strip the prefix.
    #[must_use]
    pub fn nest_no_strip(self, path: impl AsRef<str>, ep: impl IntoEndpoint) -> Self {
        self.internal_nest(&normalize_path(path.as_ref()), ep, false)
    }

    /// Nest a `Endpoint` to the specified path.
    pub fn internal_nest(mut self, path: &str, ep: impl IntoEndpoint, strip: bool) -> Self {
        let ep = Arc::new(ep.into_endpoint());
        let mut path = path.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }

        struct Nest<T> {
            inner: T,
            prefix: Option<String>,
        }

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> Self::Output {
                let idx = req.state().match_params.0.len() - 1;
                let (name, value) = req.state_mut().match_params.0.remove(idx);
                assert_eq!(name, "--poem-rest");
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query(match &self.prefix {
                            Some(prefix) => format!("{}{}", prefix, value),
                            None => format!("/{}", value),
                        })
                        .build()
                        .unwrap(),
                );
                self.inner.call(req).await.into_response()
            }
        }

        struct Root<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Root<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> Self::Output {
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query("/")
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await.into_response()
            }
        }

        assert!(
            path.find('*').is_none(),
            "wildcards are not allowed in the nest path."
        );
        self.router.add(
            &format!("{}*--poem-rest", path),
            Box::new(Nest {
                inner: ep.clone(),
                prefix: match strip {
                    false => Some(path.to_string()),
                    true => None,
                },
            }),
        );
        self.router.add(&path[..path.len() - 1], Box::new(Root(ep)));

        self
    }
}

/// Create a new routing object.
pub fn route() -> Route {
    Route {
        router: Default::default(),
    }
}

#[async_trait::async_trait]
impl Endpoint for Route {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        match self.router.recognize(req.uri().path()) {
            Ok(matches) => {
                req.state_mut().match_params.0.extend(matches.params.0);
                matches.handler.call(req).await
            }
            Err(_) => StatusCode::NOT_FOUND.into(),
        }
    }
}

/// Routing object for HTTP methods
#[derive(Default)]
pub struct RouteMethod {
    methods: FnvHashMap<Method, Box<dyn Endpoint<Output = Response>>>,
}

impl RouteMethod {
    /// Create a `RouteMethod` object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the endpoint for specified `method`.
    pub fn method(mut self, method: Method, ep: impl IntoEndpoint) -> Self {
        self.methods
            .insert(method, Box::new(ep.into_endpoint().map_to_response()));
        self
    }

    /// Sets the endpoint for `GET`.
    pub fn get(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::GET, ep)
    }

    /// Sets the endpoint for `POST`.
    pub fn post(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::POST, ep)
    }

    /// Sets the endpoint for `PUT`.
    pub fn put(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::PUT, ep)
    }

    /// Sets the endpoint for `DELETE`.
    pub fn delete(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::DELETE, ep)
    }

    /// Sets the endpoint for `HEAD`.
    pub fn head(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::HEAD, ep)
    }

    /// Sets the endpoint for `OPTIONS`.
    pub fn options(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::OPTIONS, ep)
    }

    /// Sets the endpoint for `CONNECT`.
    pub fn connect(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::CONNECT, ep)
    }

    /// Sets the endpoint for `PATCH`.
    pub fn patch(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::PATCH, ep)
    }

    /// Sets the endpoint for `TRACE`.
    pub fn trace(self, ep: impl IntoEndpoint) -> Self {
        self.method(Method::TRACE, ep)
    }
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        match self.methods.get(req.method()) {
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
pub fn get(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().get(ep)
}

/// A helper function, similar to `RouteMethod::new().post(ep)`.
pub fn post(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().post(ep)
}

/// A helper function, similar to `RouteMethod::new().put(ep)`.
pub fn put(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().put(ep)
}

/// A helper function, similar to `RouteMethod::new().delete(ep)`.
pub fn delete(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().delete(ep)
}

/// A helper function, similar to `RouteMethod::new().head(ep)`.
pub fn head(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().head(ep)
}

/// A helper function, similar to `RouteMethod::new().options(ep)`.
pub fn options(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().options(ep)
}

/// A helper function, similar to `RouteMethod::new().connect(ep)`.
pub fn connect(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().connect(ep)
}

/// A helper function, similar to `RouteMethod::new().patch(ep)`.
pub fn patch(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().patch(ep)
}

/// A helper function, similar to `RouteMethod::new().trace(ep)`.
pub fn trace(ep: impl IntoEndpoint) -> RouteMethod {
    RouteMethod::new().trace(ep)
}

fn normalize_path(path: &str) -> String {
    static RE_MERGE_SLASH: Lazy<Regex> = Lazy::new(|| Regex::new("//+").unwrap());

    let mut path = RE_MERGE_SLASH.replace_all(path, "/").to_string();
    if !path.starts_with('/') {
        path.insert(0, '/');
    }

    path
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::*;
    use crate::handler;

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
        let r = route().nest(
            "/",
            route()
                .at("/a", h)
                .at("/b", h)
                .nest("/inner", route().at("/c", h)),
        );

        assert_eq!(get(&r, "/a").await, "/a");
        assert_eq!(get(&r, "/b").await, "/b");
        assert_eq!(get(&r, "/inner/c").await, "/c");

        let r = route().nest(
            "/api",
            route()
                .at("/a", h)
                .at("/b", h)
                .nest("/inner", route().at("/c", h)),
        );

        assert_eq!(get(&r, "/api/a").await, "/a");
        assert_eq!(get(&r, "/api/b").await, "/b");
        assert_eq!(get(&r, "/api/inner/c").await, "/c");
    }

    #[tokio::test]
    async fn nested_no_strip() {
        let r = route().nest_no_strip(
            "/",
            route()
                .at("/a", h)
                .at("/b", h)
                .nest_no_strip("/inner", route().at("/inner/c", h)),
        );

        assert_eq!(get(&r, "/a").await, "/a");
        assert_eq!(get(&r, "/b").await, "/b");
        assert_eq!(get(&r, "/inner/c").await, "/inner/c");

        let r = route().nest_no_strip(
            "/api",
            route()
                .at("/api/a", h)
                .at("/api/b", h)
                .nest_no_strip("/api/inner", route().at("/api/inner/c", h)),
        );

        assert_eq!(get(&r, "/api/a").await, "/api/a");
        assert_eq!(get(&r, "/api/b").await, "/api/b");
        assert_eq!(get(&r, "/api/inner/c").await, "/api/inner/c");
    }
}
