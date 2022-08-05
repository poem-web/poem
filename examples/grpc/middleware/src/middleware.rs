use poem::{
    endpoint::{BoxEndpoint, EndpointExt},
    Endpoint, Middleware,
};

pub(crate) struct ClientMiddleware;

impl<E: Endpoint + 'static> Middleware<E> for ClientMiddleware {
    type Output = BoxEndpoint<'static, E::Output>;

    fn transform(&self, ep: E) -> Self::Output {
        ep.before(|req| async move {
            println!("client request: {}", req.uri().path());
            Ok(req)
        })
        .boxed()
    }
}

pub(crate) struct ServerMiddleware;

impl<E: Endpoint + 'static> Middleware<E> for ServerMiddleware {
    type Output = BoxEndpoint<'static, E::Output>;

    fn transform(&self, ep: E) -> Self::Output {
        ep.before(|req| async move {
            println!("handle request: {}", req.original_uri().path());
            Ok(req)
        })
        .boxed()
    }
}
