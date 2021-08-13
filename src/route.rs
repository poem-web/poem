//! Route object and DSL

use crate::endpoint::{FnHandler, FnHandlerWrapper};
use crate::error::ErrorNotFound;
use crate::method::COUNT_METHODS;
use crate::route_recognizer::Router;
use crate::{Endpoint, Error, Method, Request, Response};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Router<Box<dyn Endpoint>>,
}

impl Route {
    /// Create a new routing object.
    pub fn new() -> Self {
        Self {
            router: Default::default(),
        }
    }

    /// Add an [Endpoint] to the specified path.
    pub fn at(mut self, path: &str, ep: impl Endpoint) -> Self {
        self.router.add(path, Box::new(ep));
        self
    }
}

#[async_trait::async_trait]
impl Endpoint for Route {
    async fn call(&self, mut req: Request) -> crate::Result<Response> {
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
    /// Set a [`FnHandler`] to the [`Method::Get`].
    (get, Get);

    /// Set a [`FnHandler`] to the [`Method::Post`].
    (post, Post);

    /// Set a [`FnHandler`] to the [`Method::Put`].
    (put, Put);

    /// Set a [`FnHandler`] to the [`Method::Delete`].
    (delete, Delete);

    /// Set a [`FnHandler`] to the [`Method::Head`].
    (head, Head);

    /// Set a [`FnHandler`] to the [`Method::Options`].
    (options, Options);

    /// Set a [`FnHandler`] to the [`Method::Connect`].
    (connect, Connect);

    /// Set a [`FnHandler`] to the [`Method::Patch`].
    (patch, Patch);

    /// Set a [`FnHandler`] to the [`Method::Trace`].
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

    define_methods!(
        /// Set a [`FnHandler`] to the [`Method::Get`].
        (get, Get);

        /// Set a [`FnHandler`] to the [`Method::Post`].
        (post, Post);

        /// Set a [`FnHandler`] to the [`Method::Put`].
        (put, Put);

        /// Set a [`FnHandler`] to the [`Method::Delete`].
        (delete, Delete);

        /// Set a [`FnHandler`] to the [`Method::Head`].
        (head, Head);

        /// Set a [`FnHandler`] to the [`Method::Options`].
        (options, Options);

        /// Set a [`FnHandler`] to the [`Method::Connect`].
        (connect, Connect);

        /// Set a [`FnHandler`] to the [`Method::Patch`].
        (patch, Patch);

        /// Set a [`FnHandler`] to the [`Method::Trace`].
        (trace, Trace);
    );
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    async fn call(&self, req: Request) -> crate::Result<Response> {
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
