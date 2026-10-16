#[allow(unreachable_pub)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/method_type_harness.rs"));
}

use futures_util::{StreamExt, TryStreamExt};
use poem::{Endpoint, IntoResponse, Middleware};
use proto::{Echo, Value};

use crate::{MethodType, Request, Response, Status, Streaming};

/// Client middleware that copies the [`MethodType`] from the request
/// extensions into the `x-client-middleware` header.
pub(crate) struct ForwardMethodType;

impl<E: Endpoint> Middleware<E> for ForwardMethodType {
    type Output = ForwardMethodTypeEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ForwardMethodTypeEndpoint(ep)
    }
}

pub(crate) struct ForwardMethodTypeEndpoint<E>(E);

impl<E: Endpoint> Endpoint for ForwardMethodTypeEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: poem::Request) -> poem::Result<Self::Output> {
        if let Some(method_type) = req.extensions().get::<MethodType>().copied() {
            let _ = req
                .headers_mut()
                .insert("x-client-middleware", method_type.as_str().parse().unwrap());
        }
        self.0.call(req).await
    }
}

/// Server middleware that echoes the [`MethodType`] it sees in the request
/// extensions back in the response header named by its argument, and the
/// `x-client-middleware` request header back as is. Errors are turned into
/// responses first, so requests to unknown methods are echoed too.
pub(crate) struct EchoMethodType(pub(crate) &'static str);

impl<E: Endpoint> Middleware<E> for EchoMethodType {
    type Output = EchoMethodTypeEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        EchoMethodTypeEndpoint(ep, self.0)
    }
}

pub(crate) struct EchoMethodTypeEndpoint<E>(E, &'static str);

impl<E: Endpoint> Endpoint for EchoMethodTypeEndpoint<E> {
    type Output = poem::Response;

    async fn call(&self, req: poem::Request) -> poem::Result<Self::Output> {
        let method_type = req.extensions().get::<MethodType>().copied();
        let client = req.headers().get("x-client-middleware").cloned();
        let mut resp = match self.0.call(req).await {
            Ok(resp) => resp.into_response(),
            Err(err) => err.into_response(),
        };
        if let Some(method_type) = method_type {
            let _ = resp
                .headers_mut()
                .insert(self.1, method_type.as_str().parse().unwrap());
        }
        if let Some(client) = client {
            let _ = resp.headers_mut().insert("x-client-middleware", client);
        }
        Ok(resp)
    }
}

struct EchoService;

impl Echo for EchoService {
    async fn unary(&self, req: Request<Value>) -> Result<Response<Value>, Status> {
        Ok(Response::new(req.into_inner()))
    }

    async fn client_streaming(
        &self,
        req: Request<Streaming<Value>>,
    ) -> Result<Response<Value>, Status> {
        let values = req.into_inner().try_collect::<Vec<_>>().await?;
        Ok(Response::new(values.into_iter().next().unwrap_or_default()))
    }

    async fn server_streaming(
        &self,
        req: Request<Value>,
    ) -> Result<Response<Streaming<Value>>, Status> {
        let value = req.into_inner();
        Ok(Response::new_streaming(futures_util::stream::once(
            async move { Ok(value) },
        )))
    }

    async fn bidirectional_streaming(
        &self,
        req: Request<Streaming<Value>>,
    ) -> Result<Response<Streaming<Value>>, Status> {
        Ok(Response::new_streaming(req.into_inner()))
    }
}

struct EmptyService;

impl proto::Empty for EmptyService {}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use poem::{
        EndpointExt, IntoEndpoint,
        middleware::{NormalizePath, TrailingSlash},
    };
    use proto::{EchoClient, EchoServer, EmptyServer};

    use super::*;
    use crate::{RouteGrpc, Service};

    const ECHO: &str = "/method_type_harness.Echo";

    fn http2_request(path: &str) -> poem::Request {
        poem::Request::builder()
            .uri_str(path)
            .version(poem::http::Version::HTTP_2)
            .finish()
    }

    fn route() -> RouteGrpc {
        RouteGrpc::new()
            .add_service(EchoServer::new(EchoService))
            .add_service(EmptyServer::new(EmptyService))
    }

    /// A client calling a `RouteGrpc` with an [`EchoMethodType`] applied both
    /// with `RouteGrpc::middleware` and with `EndpointExt::with`.
    fn cli() -> EchoClient {
        let ep = route()
            .middleware(EchoMethodType("x-route-middleware"))
            .with(EchoMethodType("x-endpoint-with"));
        EchoClient::from_endpoint(ep)
    }

    /// Every documented middleware sees the method type, and a middleware
    /// wrapping the whole `RouteGrpc` sees none, even in-process.
    fn assert_method_type(metadata: &crate::Metadata, expected: MethodType) {
        for header in [
            "x-client-middleware",
            "x-route-middleware",
            "x-server-middleware",
        ] {
            assert_eq!(metadata.get(header), Some(expected.as_str()), "{header}");
        }
        assert_eq!(metadata.get("x-endpoint-with"), None);
    }

    #[tokio::test]
    async fn unary() {
        let resp = cli().unary(Request::new(Value { value: 1 })).await.unwrap();
        assert_method_type(resp.metadata(), MethodType::Unary);
    }

    #[tokio::test]
    async fn client_streaming() {
        let resp = cli()
            .client_streaming(Request::new_streaming(futures_util::stream::iter(vec![
                Ok(Value { value: 1 }),
            ])))
            .await
            .unwrap();
        assert_method_type(resp.metadata(), MethodType::ClientStreaming);
    }

    #[tokio::test]
    async fn server_streaming() {
        let resp = cli()
            .server_streaming(Request::new(Value { value: 1 }))
            .await
            .unwrap();
        assert_method_type(resp.metadata(), MethodType::ServerStreaming);
        assert_eq!(resp.into_inner().count().await, 1);
    }

    #[tokio::test]
    async fn bidirectional_streaming() {
        let resp = cli()
            .bidirectional_streaming(Request::new_streaming(futures_util::stream::iter(vec![
                Ok(Value { value: 1 }),
            ])))
            .await
            .unwrap();
        assert_method_type(resp.metadata(), MethodType::BidirectionalStreaming);
    }

    /// A `MethodType` already in the extensions is replaced by the one
    /// looked up by path, or removed if the method is unknown.
    #[tokio::test]
    async fn looked_up_by_path() {
        let ep = route()
            .middleware(EchoMethodType("x-route-middleware"))
            .into_endpoint();
        let call = |path: String| {
            let mut req = http2_request(&path);
            let _ = req
                .extensions_mut()
                .insert(MethodType::BidirectionalStreaming);
            ep.get_response(req)
        };

        let resp = call(format!("{ECHO}/ServerStreaming")).await;
        assert_eq!(
            resp.headers().get("x-route-middleware").unwrap(),
            "server_streaming"
        );
        for path in [
            format!("{ECHO}/Missing"),
            "/method_type_harness.Missing/Unary".to_string(),
        ] {
            let resp = call(path.clone()).await;
            assert_eq!(resp.headers().get("x-route-middleware"), None, "{path}");
        }
    }

    /// A `RouteGrpc` middleware rewriting the path does not leave the ones
    /// inside it with a stale `MethodType`.
    #[tokio::test]
    async fn retagged_after_path_rewrite() {
        let ep = route()
            .middleware(EchoMethodType("x-route-middleware"))
            .middleware(NormalizePath::new(TrailingSlash::Trim))
            .into_endpoint();
        let resp = ep
            .get_response(http2_request(&format!("{ECHO}/ServerStreaming/")))
            .await;
        assert_eq!(
            resp.headers().get("x-route-middleware").unwrap(),
            "server_streaming"
        );
    }

    /// The endpoint is still a `poem::Route` claiming only the service
    /// prefixes, so routes and a fallback can be added next to the services.
    #[tokio::test]
    async fn into_endpoint_keeps_route() {
        #[poem::handler]
        fn other() -> &'static str {
            "other"
        }

        let ep = route()
            .middleware(EchoMethodType("x-route-middleware"))
            .into_endpoint()
            .at("/healthz", other)
            .nest("/", other);

        let resp = ep
            .get_response(http2_request(&format!("{ECHO}/Unary")))
            .await;
        assert_eq!(resp.headers().get("x-route-middleware").unwrap(), "unary");
        for path in ["/healthz", "/ui/index.html"] {
            let resp = ep.get_response(http2_request(path)).await;
            assert_eq!(resp.into_body().into_string().await.unwrap(), "other");
        }
    }

    /// A generated server mounted without `RouteGrpc` tags requests for its
    /// server middlewares itself.
    #[tokio::test]
    async fn without_route_grpc() {
        let ep = poem::Route::new().nest(ECHO, EchoServer::new(EchoService));
        let mut req = http2_request(&format!("{ECHO}/ServerStreaming"));
        let _ = req.extensions_mut().insert(MethodType::Unary);
        let resp = ep.get_response(req).await;
        assert_eq!(
            resp.headers().get("x-server-middleware").unwrap(),
            "server_streaming"
        );
    }

    /// Built-in services are generated without server middlewares, and the
    /// reflection services combine two generated method tables.
    #[tokio::test]
    async fn builtin_services() {
        let (health, _) = crate::health_service();
        let ep = RouteGrpc::new()
            .add_service(health)
            .add_service(crate::Reflection::new().build())
            .middleware(EchoMethodType("x-route-middleware"))
            .into_endpoint();
        for (path, expected) in [
            ("/grpc.health.v1.Health/Check", MethodType::Unary),
            ("/grpc.health.v1.Health/Watch", MethodType::ServerStreaming),
            (
                "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
                MethodType::BidirectionalStreaming,
            ),
            (
                "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
                MethodType::BidirectionalStreaming,
            ),
        ] {
            let resp = ep.get_response(http2_request(path)).await;
            assert_eq!(
                resp.headers().get("x-route-middleware").unwrap(),
                expected.as_str(),
                "{path}"
            );
        }
    }

    /// Reflection served via `with_reflection` is tagged and wrapped like an
    /// added service, and the endpoint still claims only service prefixes.
    #[tokio::test]
    async fn with_reflection() {
        #[poem::handler]
        fn other() -> &'static str {
            "other"
        }

        let ep = route()
            .with_reflection(crate::Reflection::new())
            .middleware(EchoMethodType("x-route-middleware"))
            .into_endpoint()
            .nest("/", other);
        for path in [
            format!("{ECHO}/Unary"),
            "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo".to_string(),
            "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo".to_string(),
        ] {
            let resp = ep.get_response(http2_request(&path)).await;
            assert!(resp.headers().contains_key("x-route-middleware"), "{path}");
        }
        let resp = ep
            .get_response(http2_request(
                "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
            ))
            .await;
        assert_eq!(
            resp.headers().get("x-route-middleware").unwrap(),
            "bidirectional_streaming"
        );
        let resp = ep.get_response(http2_request("/ui/index.html")).await;
        assert_eq!(resp.into_body().into_string().await.unwrap(), "other");
    }

    #[test]
    fn generated_method_table() {
        assert_eq!(
            <EchoServer<EchoService> as Service>::METHODS,
            &[
                ("/method_type_harness.Echo/Unary", MethodType::Unary),
                (
                    "/method_type_harness.Echo/ClientStreaming",
                    MethodType::ClientStreaming
                ),
                (
                    "/method_type_harness.Echo/ServerStreaming",
                    MethodType::ServerStreaming
                ),
                (
                    "/method_type_harness.Echo/BidirectionalStreaming",
                    MethodType::BidirectionalStreaming
                ),
            ]
        );
    }

    /// `RouteGrpc` stays `Send + Sync`, and middlewares only need to be
    /// `Send`.
    #[test]
    fn route_grpc_is_send_sync() {
        struct NotSync(#[allow(dead_code)] Cell<u8>);

        impl<E: Endpoint> Middleware<E> for NotSync {
            type Output = E;

            fn transform(&self, ep: E) -> Self::Output {
                ep
            }
        }

        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        let route = route();
        assert_send_sync(&route);
        assert_send_sync(&route.middleware(NotSync(Cell::new(0))));
    }

    #[test]
    fn predicates() {
        for (ty, client, server, name) in [
            (MethodType::Unary, false, false, "unary"),
            (MethodType::ClientStreaming, true, false, "client_streaming"),
            (MethodType::ServerStreaming, false, true, "server_streaming"),
            (
                MethodType::BidirectionalStreaming,
                true,
                true,
                "bidirectional_streaming",
            ),
        ] {
            assert_eq!(ty.is_client_streaming(), client, "{ty:?}");
            assert_eq!(ty.is_server_streaming(), server, "{ty:?}");
            assert_eq!(ty.is_streaming(), client || server, "{ty:?}");
            assert_eq!(ty.as_str(), name);
        }
    }
}
