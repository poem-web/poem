use std::{collections::HashMap, sync::Mutex};

use futures_util::StreamExt;
use poem::{endpoint::BoxEndpoint, IntoEndpoint};
use tokio::sync::watch::{Receiver, Sender};

use crate::{Code, Request, Response, Service, Status, Streaming};

#[allow(private_in_public, unreachable_pub)]
#[allow(clippy::derive_partial_eq_without_eq)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/grpc.health.v1.rs"));
}

/// Service health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingStatus {
    /// The service is currently up and serving requests.
    Serving,
    /// The service is currently down and not serving requests.
    NotServing,
}

impl ServingStatus {
    fn to_proto(self) -> proto::health_check_response::ServingStatus {
        use proto::health_check_response::ServingStatus::*;

        match self {
            ServingStatus::Serving => Serving,
            ServingStatus::NotServing => NotServing,
        }
    }
}

type ServiceStatusMap = HashMap<String, ServingStatus>;

struct HealthService {
    receiver: Receiver<ServiceStatusMap>,
}

/// A handle providing methods to update the health status of GRPC services
pub struct HealthReporter {
    state: Mutex<(ServiceStatusMap, Sender<ServiceStatusMap>)>,
}

impl HealthReporter {
    fn set_status<S: Service>(&self, status: ServingStatus) {
        let mut state = self.state.lock().unwrap();
        state.0.insert(S::NAME.to_string(), status);
        let _ = state.1.send(state.0.clone());
    }

    /// Sets the status of the service implemented by `S` to
    /// [`ServingStatus::Serving`]
    pub fn set_serving<S: Service>(&self) {
        self.set_status::<S>(ServingStatus::Serving);
    }

    /// Sets the status of the service implemented by `S` to
    /// [`ServingStatus::NotServing`]
    pub fn set_not_serving<S: Service>(&self) {
        self.set_status::<S>(ServingStatus::NotServing);
    }

    /// Clear the status of the given service.
    pub fn clear_service_status<S: Service>(&self) {
        let mut state = self.state.lock().unwrap();
        state.0.remove(S::NAME);
        let _ = state.1.send(state.0.clone());
    }
}

#[poem::async_trait]
impl proto::Health for HealthService {
    async fn check(
        &self,
        request: Request<proto::HealthCheckRequest>,
    ) -> Result<Response<proto::HealthCheckResponse>, Status> {
        let service_status = self.receiver.borrow();
        match service_status.get(&request.service) {
            Some(status) => Ok(Response::new(proto::HealthCheckResponse {
                status: status.to_proto().into(),
            })),
            None => Err(Status::new(Code::NotFound)
                .with_message(format!("service `{}` not found", request.service))),
        }
    }

    async fn watch(
        &self,
        request: Request<proto::HealthCheckRequest>,
    ) -> Result<Response<Streaming<proto::HealthCheckResponse>>, Status> {
        let mut stream = tokio_stream::wrappers::WatchStream::new(self.receiver.clone());
        let service_name = request.into_inner().service;

        Ok(Response::new(Streaming::new(async_stream::try_stream! {
            while let Some(service_status) = stream.next().await {
                let res = service_status.get(&service_name);
                let status = res.ok_or_else(|| Status::new(Code::NotFound).with_message(format!("service `{}` not found", service_name)))?
                    .to_proto()
                    .into();
                yield proto::HealthCheckResponse { status };
            }
        })))
    }
}

/// Create health service and [`HealthReporter`]
pub fn health_service() -> (
    impl IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + Service,
    HealthReporter,
) {
    let (sender, receiver) = tokio::sync::watch::channel(Default::default());

    (
        proto::HealthServer::new(HealthService { receiver }),
        HealthReporter {
            state: Mutex::new((Default::default(), sender)),
        },
    )
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;
    use crate::health::proto::Health;

    fn create_service() -> (HealthService, HealthReporter) {
        let (sender, receiver) = tokio::sync::watch::channel(Default::default());
        (
            HealthService { receiver },
            HealthReporter {
                state: Mutex::new((Default::default(), sender)),
            },
        )
    }

    #[tokio::test]
    async fn check() {
        let (service, reporter) = create_service();

        let res = service
            .check(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await;
        assert_eq!(res.unwrap_err().code(), Code::NotFound);

        reporter.set_serving::<proto::HealthServer<HealthService>>();
        let res = service
            .check(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await;
        assert_eq!(
            res.unwrap().into_inner(),
            proto::HealthCheckResponse {
                status: proto::health_check_response::ServingStatus::Serving.into()
            }
        );

        reporter.set_not_serving::<proto::HealthServer<HealthService>>();
        let res = service
            .check(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await;
        assert_eq!(
            res.unwrap().into_inner(),
            proto::HealthCheckResponse {
                status: proto::health_check_response::ServingStatus::NotServing.into()
            }
        );

        reporter.clear_service_status::<proto::HealthServer<HealthService>>();
        let res = service
            .check(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await;
        assert_eq!(res.unwrap_err().code(), Code::NotFound);
    }

    #[tokio::test]
    async fn watch() {
        let (service, reporter) = create_service();

        let mut stream = service
            .watch(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(
            stream.next().await.unwrap().unwrap_err().code(),
            Code::NotFound
        );

        reporter.set_serving::<proto::HealthServer<HealthService>>();
        let mut stream = service
            .watch(Request::new(proto::HealthCheckRequest {
                service: <proto::HealthServer<HealthService>>::NAME.to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            proto::HealthCheckResponse {
                status: proto::health_check_response::ServingStatus::Serving.into()
            }
        );

        reporter.set_not_serving::<proto::HealthServer<HealthService>>();
        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            proto::HealthCheckResponse {
                status: proto::health_check_response::ServingStatus::NotServing.into()
            }
        );

        reporter.clear_service_status::<proto::HealthServer<HealthService>>();
        assert_eq!(
            stream.next().await.unwrap().unwrap_err().code(),
            Code::NotFound
        );
    }
}
