#[allow(unreachable_pub)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/test_harness.rs"));
}

use futures_util::TryStreamExt;
use proto::{TestHarness, UnaryRequest, ValueRequest, ValueResponse};

use crate::{Request, Response, Status, Streaming};

pub(crate) struct TestHarnessService;

impl TestHarness for TestHarnessService {
    async fn unary(&self, req: Request<UnaryRequest>) -> Result<Response<ValueResponse>, Status> {
        Ok(Response::new(ValueResponse {
            value: req.a + req.b,
        }))
    }

    async fn client_streaming(
        &self,
        req: Request<Streaming<ValueRequest>>,
    ) -> Result<Response<ValueResponse>, Status> {
        Ok(Response::new(ValueResponse {
            value: req
                .into_inner()
                .try_fold(0, |acc, req| async move { Ok(acc + req.value) })
                .await?,
        }))
    }

    async fn server_streaming(
        &self,
        req: Request<ValueRequest>,
    ) -> Result<Response<Streaming<ValueResponse>>, Status> {
        Ok(Response::new_streaming(futures_util::stream::try_unfold(
            req.value,
            |state| async move {
                Ok(if state >= 0 {
                    Some((ValueResponse { value: state }, state - 1))
                } else {
                    None
                })
            },
        )))
    }

    async fn bidirectional_streaming(
        &self,
        req: Request<Streaming<ValueRequest>>,
    ) -> Result<Response<Streaming<ValueResponse>>, Status> {
        let mut stream = req.into_inner();
        Ok(Response::new_streaming(async_stream::try_stream! {
            let mut sum = 0;
            while let Some(ValueRequest { value }) = stream.try_next().await? {
                sum += value;
                yield ValueResponse { value: sum };
            }
        }))
    }

    async fn unary_metadata(
        &self,
        req: Request<UnaryRequest>,
    ) -> Result<Response<ValueResponse>, Status> {
        let mut resp = Response::new(ValueResponse {
            value: req.a + req.b,
        });
        if let Some(value) = req.metadata().get("mydata") {
            resp.metadata_mut().insert("mydata", value);
        }
        if let Some(method_type) = req.extensions().get::<crate::MethodType>() {
            resp.metadata_mut()
                .insert("x-method-type", method_type.as_str());
        }
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytes::{BufMut, Bytes, BytesMut};
    use futures_util::stream::StreamExt;
    use poem::{
        Body, Endpoint, IntoEndpoint,
        http::{Method, Version},
    };
    use prost::Message;
    use proto::{TestHarnessClient, TestHarnessServer};

    use super::*;
    use crate::{Code, RouteGrpc};

    fn create_cli() -> TestHarnessClient {
        let server = TestHarnessServer::new(TestHarnessService);
        let route = RouteGrpc::new().add_service(server);
        TestHarnessClient::from_endpoint(route)
    }

    #[tokio::test]
    async fn unary() {
        let cli = create_cli();
        let resp = cli
            .unary(Request::new(UnaryRequest { a: 10, b: 20 }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner(), ValueResponse { value: 30 });
    }

    #[tokio::test]
    async fn client_streaming() {
        let cli = create_cli();
        let resp = cli
            .client_streaming(Request::new_streaming(
                futures_util::stream::iter(vec![10, 20, 30])
                    .map(|value| Ok(ValueRequest { value })),
            ))
            .await
            .unwrap();
        assert_eq!(resp.into_inner(), ValueResponse { value: 60 });
    }

    #[tokio::test]
    async fn server_streaming() {
        let cli = create_cli();
        let resp = cli
            .server_streaming(Request::new(ValueRequest { value: 5 }))
            .await
            .unwrap();
        assert_eq!(
            resp.into_inner()
                .map_ok(|resp| resp.value)
                .try_collect::<Vec<_>>()
                .await
                .unwrap(),
            vec![5, 4, 3, 2, 1, 0]
        );
    }

    #[tokio::test]
    async fn bidirectional_streaming() {
        let cli = create_cli();
        let resp = cli
            .bidirectional_streaming(Request::new_streaming(
                futures_util::stream::iter(vec![10, 20, 30])
                    .map(|value| Ok(ValueRequest { value })),
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.into_inner()
                .map_ok(|resp| resp.value)
                .try_collect::<Vec<_>>()
                .await
                .unwrap(),
            vec![10, 30, 60]
        );
    }

    #[tokio::test]
    async fn metadata() {
        let cli = create_cli();
        let mut req = Request::new(UnaryRequest { a: 10, b: 20 });
        req.metadata_mut().insert("mydata", "abc");
        let resp = cli.unary_metadata(req).await.unwrap();
        assert_eq!(resp.metadata().get("mydata"), Some("abc"));
        assert_eq!(resp.into_inner(), ValueResponse { value: 30 });
    }

    fn data_frame(msg: impl Message) -> Bytes {
        let msg = msg.encode_to_vec();
        let mut buf = BytesMut::new();
        buf.put_u8(0);
        buf.put_u32(msg.len() as u32);
        buf.extend(msg);
        buf.freeze()
    }

    fn request(method: &str, body: Body) -> poem::Request {
        poem::Request::builder()
            .method(Method::POST)
            .version(Version::HTTP_2)
            .uri_str(format!("/test_harness.TestHarness/{method}"))
            .content_type("application/grpc")
            .body(body)
    }

    /// The server responds only once the request stream has ended, so the
    /// client's final DATA frame doesn't arrive on a closed stream.
    #[tokio::test]
    async fn unary_waits_for_request_end() {
        let route = RouteGrpc::new()
            .add_service(TestHarnessServer::new(TestHarnessService))
            .into_endpoint();
        let (end_tx, end_rx) = tokio::sync::oneshot::channel::<()>();
        let message = data_frame(UnaryRequest { a: 10, b: 20 });
        let body = Body::from_bytes_stream(
            futures_util::stream::once(async move { Ok::<_, std::io::Error>(message) }).chain(
                futures_util::stream::once(async move {
                    let _ = end_rx.await;
                    Ok(Bytes::new())
                }),
            ),
        );

        let mut resp = Box::pin(route.get_response(request("Unary", body)));
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut resp)
                .await
                .is_err()
        );
        end_tx.send(()).unwrap();
        let resp = resp.await;
        assert_eq!(resp.headers().get("grpc-status"), None);
    }

    #[tokio::test]
    async fn unary_rejects_extra_request_message() {
        let message = data_frame(UnaryRequest { a: 10, b: 20 });
        let body = Body::from_bytes([message.clone(), message].concat().into());
        let status = status_of("Unary", body).await;
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), Some("expected only one request message"));
    }

    async fn status_of(method: &str, body: Body) -> crate::Status {
        let route = RouteGrpc::new()
            .add_service(TestHarnessServer::new(TestHarnessService))
            .into_endpoint();
        let resp = route.get_response(request(method, body)).await;
        crate::Status::from_headers(resp.headers())
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn missing_request_message() {
        for method in ["Unary", "ServerStreaming"] {
            let status = status_of(method, Body::empty()).await;
            assert_eq!(status.code(), Code::Internal);
            assert_eq!(status.message(), Some("missing request message"));
        }
    }

    /// A partial message after the request message fails the call.
    #[tokio::test]
    async fn incomplete_request() {
        let message = data_frame(UnaryRequest { a: 10, b: 20 });
        let body = Body::from_bytes([message.clone(), message.slice(..6)].concat().into());
        assert_eq!(status_of("Unary", body).await.code(), Code::Internal);
    }

    /// Compressed requests of non-client-streaming calls.
    #[cfg(feature = "gzip")]
    #[tokio::test]
    async fn compressed_request() {
        use crate::CompressionEncoding;

        let server = TestHarnessServer::new(TestHarnessService)
            .accept_compressed([CompressionEncoding::GZIP]);
        let mut cli = TestHarnessClient::from_endpoint(RouteGrpc::new().add_service(server));
        cli.set_send_compressed(CompressionEncoding::GZIP);

        let resp = cli
            .unary(Request::new(UnaryRequest { a: 10, b: 20 }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner(), ValueResponse { value: 30 });

        let resp = cli
            .server_streaming(Request::new(ValueRequest { value: 2 }))
            .await
            .unwrap();
        assert_eq!(
            resp.into_inner()
                .map_ok(|resp| resp.value)
                .try_collect::<Vec<_>>()
                .await
                .unwrap(),
            vec![2, 1, 0]
        );
    }
    /// Handlers see the `MethodType` without any middleware configured,
    /// whether the server is added to a `RouteGrpc` or mounted directly. The
    /// in-process client does not pass its own one to the server.
    #[tokio::test]
    async fn method_type_in_handler() {
        let clients = [
            create_cli(),
            TestHarnessClient::from_endpoint(poem::Route::new().nest(
                "/test_harness.TestHarness",
                TestHarnessServer::new(TestHarnessService),
            )),
        ];
        for cli in clients {
            let resp = cli
                .unary_metadata(Request::new(UnaryRequest { a: 1, b: 2 }))
                .await
                .unwrap();
            assert_eq!(resp.metadata().get("x-method-type"), Some("unary"));
        }
    }
}
