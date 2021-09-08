//! Route object and DSL

use std::sync::Arc;

use fnv::FnvHashMap;
use http::StatusCode;

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
    pub fn at(mut self, path: &str, ep: impl IntoEndpoint) -> Self {
        self.router
            .add(path, Box::new(ep.into_endpoint().map_to_response()));
        self
    }

    /// Nest a `Endpoint` to the specified path.
    #[must_use]
    pub fn nest(mut self, path: &str, ep: impl IntoEndpoint) -> Self {
        let ep = Arc::new(ep.into_endpoint());
        let path = path.trim_end_matches('/');

        struct Nest<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> Self::Output {
                let idx = req.state().match_params.0.len() - 1;
                let (name, value) = req.state_mut().match_params.0.remove(idx);
                assert_eq!(name, "--poem-rest");
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query(value)
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await.into_response()
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
            &format!("{}/*--poem-rest", path),
            Box::new(Nest(ep.clone())),
        );
        self.router.add(path, Box::new(Root(ep)));

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
