use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use poem::{EndpointExt, IntoEndpoint, Middleware, Response, endpoint::BoxEndpoint};

use crate::{MethodType, Reflection, Service, method_type::set_method_type};

type BoxedMiddleware =
    Box<dyn FnOnce(BoxEndpoint<'static, Response>) -> BoxEndpoint<'static, Response> + Send>;

/// A router for GRPC services
///
/// Middlewares applied with [`RouteGrpc::middleware`] can read the
/// [`MethodType`] of the called method from the request extensions.
#[derive(Default)]
pub struct RouteGrpc {
    route: poem::Route,
    /// `Service::NAME` of every service added via `add_service`.
    service_names: Vec<&'static str>,
    reflection: Option<Reflection>,
    method_types: HashMap<&'static str, MethodType>,
    // `Mutex` keeps `RouteGrpc: Sync` without requiring middlewares to be
    // `Sync`; it is only locked once, in `into_endpoint`.
    middlewares: Vec<Mutex<BoxedMiddleware>>,
}

impl RouteGrpc {
    /// Create a `GrpcRoute`
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a GRPC service
    ///
    /// The [`MethodType`]s of its methods are taken from
    /// [`Service::METHODS`].
    #[must_use]
    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoEndpoint<Endpoint = BoxEndpoint<'static, Response>> + Service,
    {
        self.service_names.push(S::NAME);
        self.method_types.extend(S::METHODS.iter().copied());
        self.route = self.route.nest(format!("/{}", S::NAME), service);
        self
    }

    /// Serve reflection, advertising the services registered on this router.
    ///
    /// `ListServices` returns the services added via
    /// [`add_service`](Self::add_service) plus reflection itself, instead of
    /// every service found in the descriptors (which is what adding
    /// `reflection.build()` via `add_service` does). A mounted service is
    /// listed only if the registered descriptors define it under its
    /// [`Service::NAME`], since otherwise it could not be described. Symbol
    /// and file lookups are unaffected.
    ///
    /// [`Reflection::advertise_services`] narrows this list down further.
    /// **Reflection itself is then listed only if its names were included.**
    ///
    /// The reflection service is built when the router is turned into an
    /// endpoint, so the order relative to `add_service` does not matter.
    /// Calling this again replaces the previous reflection builder.
    ///
    /// # Panics
    ///
    /// When the router is turned into an endpoint, if a service added via
    /// [`add_service`](Self::add_service) would clash with the reflection
    /// service: one with an empty [`Service::NAME`] (such as
    /// `Reflection::build()`), which is mounted at `/` and overlaps reflection,
    /// or one named like an entry of [`Reflection::SERVICE_NAMES`].
    #[must_use]
    pub fn with_reflection(mut self, reflection: Reflection) -> Self {
        self.reflection = Some(reflection);
        self
    }

    /// Apply a middleware to all GRPC services.
    ///
    /// Unlike [`EndpointExt::with`], which wraps the whole `RouteGrpc`, a
    /// middleware applied here can read the [`MethodType`] of the called
    /// method from the request extensions.
    ///
    /// Like chained [`EndpointExt::with`] calls, a middleware applied later
    /// wraps the ones applied before it, so it sees the request first.
    ///
    /// ```
    /// use poem::middleware::Tracing;
    /// use poem_grpc::RouteGrpc;
    ///
    /// let route = RouteGrpc::new().middleware(Tracing);
    /// ```
    #[must_use]
    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<BoxEndpoint<'static, Response>> + Send + 'static,
        M::Output: 'static,
    {
        self.middlewares.push(Mutex::new(Box::new(move |ep| {
            middleware.transform(ep).map_to_response().boxed()
        })));
        self
    }
}

impl IntoEndpoint for RouteGrpc {
    type Endpoint = poem::Route;

    fn into_endpoint(mut self) -> Self::Endpoint {
        if let Some(reflection) = self.reflection.take() {
            for name in &self.service_names {
                assert!(
                    !name.is_empty(),
                    "RouteGrpc::with_reflection: a service with an empty `Service::NAME` (such as \
                     `Reflection::build()`) is added via `add_service`; it would be mounted at `/`, \
                     overlapping reflection"
                );
                assert!(
                    !Reflection::SERVICE_NAMES.contains(name),
                    "RouteGrpc::with_reflection: `{name}` is also added via `add_service`"
                );
            }
            let reflection = reflection
                .mounted_on_router(self.service_names.iter().copied())
                .build();
            self.method_types.extend(service_methods(&reflection));
            // Mount it at the prefix of each reflection service rather than at
            // `/`, like the endpoint returned below, which mounts the route
            // without stripping these prefixes.
            let reflection = Arc::new(reflection.into_endpoint());
            for name in Reflection::SERVICE_NAMES {
                self.service_names.push(name);
                self.route = self
                    .route
                    .nest_no_strip(format!("/{name}"), reflection.clone());
            }
        }

        // Without middlewares, nothing would read the `MethodType` tagged here:
        // generated servers tag requests for their own middlewares and
        // handlers. Skip the tagging layer and the `Route` mounting it.
        if self.middlewares.is_empty() {
            return self.route;
        }

        let method_types = Arc::new(self.method_types);
        let tag = |ep: BoxEndpoint<'static, Response>| {
            let method_types = method_types.clone();
            ep.before(move |mut req| {
                let method_type = method_types.get(req.uri().path()).copied();
                set_method_type(&mut req, method_type);
                std::future::ready(Ok(req))
            })
            .boxed()
        };

        // Tag the request in front of the route and of every middleware, so
        // one rewriting the request path does not leave the middlewares inside
        // it with a stale `MethodType`.
        let mut ep = tag(self.route.boxed());
        for middleware in self.middlewares {
            let middleware = middleware
                .into_inner()
                .unwrap_or_else(|err| err.into_inner());
            ep = tag(middleware(ep));
        }

        // Mount the wrapped endpoint at the prefix of each service, so the
        // returned `Route` claims the same paths as the services themselves
        // and other routes can still be added next to them.
        let ep = Arc::new(ep);
        self.service_names
            .iter()
            .fold(poem::Route::new(), |route, name| {
                route.nest_no_strip(format!("/{name}"), ep.clone())
            })
    }
}

fn service_methods<S: Service>(_: &S) -> impl Iterator<Item = (&'static str, MethodType)> {
    S::METHODS.iter().copied()
}
