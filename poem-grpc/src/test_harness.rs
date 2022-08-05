#[allow(private_in_public, unreachable_pub)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/test_harness.rs"));
}

use futures_util::TryStreamExt;
use proto::{TestHarness, UnaryRequest, ValueRequest, ValueResponse};

use crate::{Request, Response, Status, Streaming};

pub(crate) struct TestHarnessService;

#[poem::async_trait]
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
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use futures_util::stream::StreamExt;
    use proto::{TestHarnessClient, TestHarnessServer};

    use super::*;
    use crate::RouteGrpc;

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
}
