//! Route object and DSL

use std::{collections::HashMap, sync::Arc};

use crate::{
    error::ErrorNotFound, http::Method, route_recognizer::Router, Endpoint, Error, Request,
    Response, Result,
};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Router<Box<dyn Endpoint>>,
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
    /// use poem::{get, handler, route, web::Path};
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
    /// let app = route()
    ///     // full path
    ///     .at("/a/b", get(a))
    ///     // capture parameters
    ///     .at("/b/:group/:name", get(b))
    ///     // capture tail path
    ///     .at("/c/*path", get(c));
    /// ```
    pub fn at(mut self, path: &str, ep: impl Endpoint) -> Self {
        self.router.add(path, Box::new(ep));
        self
    }

    /// Nest a `Endpoint` to the specified path.
    pub fn nest(mut self, path: &str, ep: impl Endpoint) -> Self {
        let ep = Arc::new(ep);
        let path = path.trim_end_matches('/');

        struct Nest<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            async fn call(&self, mut req: Request) -> Result<Response> {
                let idx = req.state().match_params.0.len() - 1;
                let (name, value) = req.state_mut().match_params.0.remove(idx);
                assert_eq!(name, "--poem-rest");
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query(value)
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await
            }
        }

        struct Root<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Root<E> {
            async fn call(&self, mut req: Request) -> Result<Response> {
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query("/")
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await
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
    async fn call(&self, mut req: Request) -> Result<Response> {
        let m = self
            .router
            .recognize(req.uri().path())
            .ok()
            .ok_or_else(|| Into::<Error>::into(ErrorNotFound))?;
        req.state_mut().match_params.0.extend(m.params.0);
        m.handler.call(req).await
    }
}

macro_rules! define_method_fn {
    ($($(#[$docs:meta])* ($name:ident, $method:ident);)*) => {
        $(
        $(#[$docs])*
        pub fn $name(ep: impl Endpoint) -> RouteMethod {
            let mut router = RouteMethod::default();
            router.router.insert(Method::$method, Box::new(ep));
            router
        }
        )*
    };
}

define_method_fn!(
    /// Set a handler to the `GET` and returns endpoint [`RouteMethod`].
    (get, GET);

    /// Set a handler to the `POST` and returns endpoint [`RouteMethod`].
    (post, POST);

    /// Set a handler to the `PUT` and returns [`RouteMethod`].
    (put, PUT);

    /// Set a handler to the `DELETE` and returns [`RouteMethod`].
    (delete, DELETE);

    /// Set a handler to the `HEAD` and returns [`RouteMethod`].
    (head, HEAD);

    /// Set a handler to the `OPTIONS` and returns [`RouteMethod`].
    (options, OPTIONS);

    /// Set a handler to the `CONNECT` and returns [`RouteMethod`].
    (connect, CONNECT);

    /// Set a handler to the `PATCH` and returns [`RouteMethod`].
    (patch, PATCH);

    /// Set a handler to the `TRACE` and returns [`RouteMethod`].
    (trace, TRACE);
);

macro_rules! define_methods {
    ($($(#[$docs:meta])* ($name:ident, $method:ident);)*) => {
        $(
        $(#[$docs])*
        pub fn $name(mut self, ep: impl Endpoint) -> Self {
            self.router.insert(Method::$method, Box::new(ep));
            self
        }
        )*
    };
}

/// HTTP methods routing object.
#[derive(Default)]
pub struct RouteMethod {
    router: HashMap<Method, Box<dyn Endpoint>>,
    any_router: Option<Box<dyn Endpoint>>,
}

impl RouteMethod {
    /// Set a [`Endpoint`] to the specified method type.
    pub fn method(mut self, method: Method, ep: impl Endpoint) -> Self {
        self.router.insert(method, Box::new(ep));
        self
    }

    /// Set [`Endpoint`] to all method types.
    pub fn any(mut self, ep: impl Endpoint) -> Self {
        self.any_router = Some(Box::new(ep));
        self
    }

    define_methods!(
        /// Set a handler to the `GET`.
        (get, GET);

        /// Set a handler to the `POST`.
        (post, POST);

        /// Set a handler to the `PUT`.
        (put, PUT);

        /// Set a handler to the `DELETE`.
        (delete, DELETE);

        /// Set a handler to the `HEAD`.
        (head, HEAD);

        /// Set a handler to the `OPTIONS`.
        (options, OPTIONS);

        /// Set a handler to the `CONNECT`.
        (connect, CONNECT);

        /// Set a handler to the `PATCH`.
        (patch, PATCH);

        /// Set a handler to the `TRACE`.
        (trace, TRACE);
    );
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    async fn call(&self, req: Request) -> Result<Response> {
        if req.method() == Method::HEAD {
            let ep = self
                .router
                .get(&Method::GET)
                .or_else(|| self.any_router.as_ref());
            return if let Some(ep) = ep {
                let mut resp = ep.call(req).await?;
                let _ = resp.take_body();
                Ok(resp)
            } else {
                Err(ErrorNotFound.into())
            };
        }

        if let Some(ep) = &self.any_router {
            return ep.call(req).await;
        }

        if let Some(ep) = self.router.get(req.method()) {
            ep.call(req).await
        } else {
            Err(ErrorNotFound.into())
        }
    }
}
