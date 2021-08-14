//! Route object and DSL

use crate::endpoint::{FnHandler, FnHandlerWrapper};
use crate::error::ErrorNotFound;
use crate::method::COUNT_METHODS;
use crate::route_recognizer::Router;
use crate::{Endpoint, Error, Method, Request, Response, Result};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Router<Box<dyn Endpoint>>,
}

impl Route {
    /// Add an [Endpoint] to the specified path.
    pub fn at(mut self, path: &str, ep: impl Endpoint) -> Self {
        self.router.add(path, Box::new(ep));
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
            .ok_or_else(|| Error::not_found(ErrorNotFound))?;
        req.extensions_mut().insert(m.params);
        m.handler.call(req).await
    }
}

macro_rules! define_method_fn {
    ($($(#[$docs:meta])* ($name:ident, $method:ident);)*) => {
        $(
        $(#[$docs])*
        pub fn $name<T, In>(ep: T) -> RouteMethod
        where
            T: FnHandler<In> + 'static,
            In: Send + Sync + 'static,
        {
            let mut router = RouteMethod::default();
            router.router[Method::$method as usize] = Some(Box::new(FnHandlerWrapper::new(ep)) as Box<dyn Endpoint>);
            router
        }
        )*
    };
}

define_method_fn!(
    /// Set a handler to the `GET` and returns [`RouteMethod`].
    (get, Get);

    /// Set a handler to the `POST` and returns [`RouteMethod`].
    (post, Post);

    /// Set a handler to the `PUT` and returns [`RouteMethod`].
    (put, Put);

    /// Set a handler to the `DELETE` and returns [`RouteMethod`].
    (delete, Delete);

    /// Set a handler to the `HEAD` and returns [`RouteMethod`].
    (head, Head);

    /// Set a handler to the `OPTIONS` and returns [`RouteMethod`].
    (options, Options);

    /// Set a handler to the `CONNECT` and returns [`RouteMethod`].
    (connect, Connect);

    /// Set a handler to the `PATCH` and returns [`RouteMethod`].
    (patch, Patch);

    /// Set a handler to the `TRACE` and returns [`RouteMethod`].
    (trace, Trace);
);

macro_rules! define_methods {
    ($($(#[$docs:meta])* ($name:ident, $method:ident);)*) => {
        $(
        $(#[$docs])*
        pub fn $name<T, In>(mut self, ep: T) -> Self
        where
            T: FnHandler<In> + 'static,
            In: Send + Sync + 'static,
        {
            self.router[Method::$method as usize] = Some(Box::new(FnHandlerWrapper::new(ep)));
            self
        }
        )*
    };
}

/// HTTP methods routing object.
#[derive(Default)]
pub struct RouteMethod {
    router: [Option<Box<dyn Endpoint>>; COUNT_METHODS],
    any_router: Option<Box<dyn Endpoint>>,
}

impl RouteMethod {
    /// Set a [`FnHandler`] to the specified method type.
    pub fn method<T, In>(mut self, method: Method, ep: T) -> Self
    where
        T: FnHandler<In> + 'static,
        In: Send + Sync + 'static,
    {
        self.router[method as usize] = Some(Box::new(FnHandlerWrapper::new(ep)));
        self
    }

    /// Set [`FnHandler`] to all method types.
    pub fn any<T, In>(mut self, ep: T) -> Self
    where
        T: FnHandler<In> + 'static,
        In: Send + Sync + 'static,
    {
        self.any_router = Some(Box::new(FnHandlerWrapper::new(ep)));
        self
    }

    define_methods!(
        /// Set a handler to the `GET`.
        (get, Get);

        /// Set a handler to the `POST`.
        (post, Post);

        /// Set a handler to the `PUT`.
        (put, Put);

        /// Set a handler to the `DELETE`.
        (delete, Delete);

        /// Set a handler to the `HEAD`.
        (head, Head);

        /// Set a handler to the `OPTIONS`.
        (options, Options);

        /// Set a handler to the `CONNECT`.
        (connect, Connect);

        /// Set a handler to the `PATCH`.
        (patch, Patch);

        /// Set a handler to the `TRACE`.
        (trace, Trace);
    );
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    async fn call(&self, req: Request) -> Result<Response> {
        if let Some(ep) = &self.any_router {
            return ep.call(req).await;
        }

        if let Some(ep) = self
            .router
            .get(req.method() as usize)
            .and_then(|ep| ep.as_ref())
        {
            ep.call(req).await
        } else {
            Err(Error::not_found(ErrorNotFound))
        }
    }
}
