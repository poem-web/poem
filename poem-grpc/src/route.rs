use poem::{IntoEndpoint, Response, endpoint::BoxEndpoint};

use crate::{Reflection, Service};

/// A router for GRPC services
#[derive(Default)]
pub struct RouteGrpc {
    route: poem::Route,
    /// `Service::NAME` of every service added via `add_service`.
    service_names: Vec<&'static str>,
    reflection: Option<Reflection>,
}

impl RouteGrpc {
    /// Create a `GrpcRoute`
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a GRPC service
    #[must_use]
    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoEndpoint<Endpoint = BoxEndpoint<'static, Response>> + Service,
    {
        self.service_names.push(S::NAME);
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
    /// `Reflection::build()`), which is mounted at `/` like reflection, or
    /// one named like an entry of [`Reflection::SERVICE_NAMES`].
    #[must_use]
    pub fn with_reflection(mut self, reflection: Reflection) -> Self {
        self.reflection = Some(reflection);
        self
    }
}

impl IntoEndpoint for RouteGrpc {
    type Endpoint = poem::Route;

    fn into_endpoint(self) -> Self::Endpoint {
        let Some(reflection) = self.reflection else {
            return self.route;
        };

        for name in &self.service_names {
            assert!(
                !name.is_empty(),
                "RouteGrpc::with_reflection: a service with an empty `Service::NAME` (such as \
                 `Reflection::build()`) is added via `add_service`; it would be mounted at `/`, \
                 where reflection is mounted"
            );
            assert!(
                !Reflection::SERVICE_NAMES.contains(name),
                "RouteGrpc::with_reflection: `{name}` is also added via `add_service`"
            );
        }
        let reflection = reflection
            .mounted_on_router(self.service_names.iter().copied())
            .build();
        self.route.nest("/", reflection)
    }
}
