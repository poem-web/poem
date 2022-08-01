use poem::{endpoint::BoxEndpoint, IntoEndpoint, Response};

use crate::Service;

/// A router for GRPC services
#[derive(Default)]
pub struct Route {
    route: poem::Route,
}

impl Route {
    /// Create a `GrpcRoute`
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a GRPC service
    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoEndpoint<Endpoint = BoxEndpoint<'static, Response>> + Service,
    {
        self.route = self.route.nest(format!("/{}", S::name()), service);
        self
    }
}

impl IntoEndpoint for Route {
    type Endpoint = poem::Route;

    fn into_endpoint(self) -> Self::Endpoint {
        self.route
    }
}
