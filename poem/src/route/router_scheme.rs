use http::uri::Scheme;

use crate::{
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, endpoint::BoxEndpoint,
    error::NotFoundError,
};

/// Routing object for request scheme
///
/// # Errors
///
/// - [`NotFoundError`]
#[derive(Default)]
pub struct RouteScheme {
    schemes: Vec<(Scheme, BoxEndpoint<'static>)>,
    fallback: Option<BoxEndpoint<'static>>,
}

impl RouteScheme {
    /// Create a `RouteScheme` object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the endpoint for `HTTPS`.
    #[must_use]
    pub fn https<E>(mut self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.schemes
            .push((Scheme::HTTPS, ep.into_endpoint().map_to_response().boxed()));
        self
    }

    /// Sets the endpoint for `HTTP`.
    #[must_use]
    pub fn http<E>(mut self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.schemes
            .push((Scheme::HTTP, ep.into_endpoint().map_to_response().boxed()));
        self
    }

    /// Sets the endpoint for the specified `scheme`.
    #[must_use]
    pub fn custom<E>(mut self, scheme: Scheme, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.schemes
            .push((scheme, ep.into_endpoint().map_to_response().boxed()));
        self
    }

    /// Sets the endpoint for the fallback schemes.
    ///
    /// All unmatched schemes will use this endpoint.
    #[must_use]
    pub fn fallback<E>(mut self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.fallback = Some(ep.into_endpoint().map_to_response().boxed());
        self
    }
}

impl Endpoint for RouteScheme {
    type Output = Response;

    async fn call(&self, req: Request) -> crate::Result<Self::Output> {
        match self
            .schemes
            .iter()
            .find(|(scheme, _)| scheme == req.scheme())
            .map(|(_, ep)| ep)
        {
            Some(ep) => ep.call(req).await,
            None => match &self.fallback {
                Some(ep) => ep.call(req).await,
                None => Err(NotFoundError.into()),
            },
        }
    }
}
