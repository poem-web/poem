//! Route object and DSL

use std::{collections::HashMap, sync::Arc};

use http::StatusCode;

use crate::{
    http::Method, route_recognizer::Router, Endpoint, EndpointExt, IntoEndpoint, IntoResponse,
    Request, Response,
};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: HashMap<Method, Router<Box<dyn Endpoint<Output = Response>>>>,
    all_method_router: Router<Box<dyn Endpoint<Output = Response>>>,
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
    /// use poem::{handler, route, web::Path};
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
    /// let mut app = route();
    ///
    /// // full path
    /// app.at("/a/b").get(a);
    ///
    /// // capture parameters
    /// app.at("/b/:group/:name").get(b);
    ///
    /// // capture tail path
    /// app.at("/c/*path").get(c);
    /// ```
    pub fn at<'a>(&'a mut self, path: &'a str) -> RouteMethod<'a> {
        RouteMethod { path, router: self }
    }

    /// Nest a `Endpoint` to the specified path.
    pub fn nest(&mut self, path: &str, ep: impl IntoEndpoint) {
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
        self.all_method_router.add(
            &format!("{}/*--poem-rest", path),
            Box::new(Nest(ep.clone())),
        );
        self.all_method_router.add(path, Box::new(Root(ep)));
    }
}

/// Create a new routing object.
pub fn route() -> Route {
    Route {
        router: Default::default(),
        all_method_router: Default::default(),
    }
}

#[async_trait::async_trait]
impl Endpoint for Route {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Self::Output {
        match self
            .router
            .get(req.method())
            .and_then(|router| router.recognize(req.uri().path()).ok())
            .or_else(|| self.all_method_router.recognize(req.uri().path()).ok())
        {
            Some(matches) => {
                req.state_mut().match_params.0.extend(matches.params.0);
                matches.handler.call(req).await
            }
            None => StatusCode::NOT_FOUND.into(),
        }
    }
}

/// Used to set the endpoint of the HTTP methods.
pub struct RouteMethod<'a> {
    path: &'a str,
    router: &'a mut Route,
}

impl<'a> RouteMethod<'a> {
    /// Sets the endpoint for specified `method`.
    pub fn method(&mut self, method: Method, ep: impl IntoEndpoint) -> &mut Self {
        self.router
            .router
            .entry(method)
            .or_default()
            .add(self.path, Box::new(ep.into_endpoint().map_to_response()));
        self
    }

    /// Sets the endpoint for `GET`.
    pub fn get(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::GET, ep)
    }

    /// Sets the endpoint for `POST`.
    pub fn post(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::POST, ep)
    }

    /// Sets the endpoint for `PUT`.
    pub fn put(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::PUT, ep)
    }

    /// Sets the endpoint for `DELETE`.
    pub fn delete(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::DELETE, ep)
    }

    /// Sets the endpoint for `HEAD`.
    pub fn head(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::HEAD, ep)
    }

    /// Sets the endpoint for `OPTIONS`.
    pub fn options(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::OPTIONS, ep)
    }

    /// Sets the endpoint for `CONNECT`.
    pub fn connect(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::CONNECT, ep)
    }

    /// Sets the endpoint for `PATCH`.
    pub fn patch(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::PATCH, ep)
    }

    /// Sets the endpoint for `TRACE`.
    pub fn trace(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.method(Method::TRACE, ep)
    }

    /// Sets the endpoint for all methods.
    pub fn all(&mut self, ep: impl IntoEndpoint) -> &mut Self {
        self.router
            .all_method_router
            .add(self.path, Box::new(ep.into_endpoint().map_to_response()));
        self
    }
}
