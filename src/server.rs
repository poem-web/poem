use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{Endpoint, Request};

pub struct Server {
    ep: Arc<dyn Endpoint>,
}

impl Server {
    pub fn new(root: impl Endpoint) -> Self {
        Server { ep: Arc::new(root) }
    }

    pub async fn serve(self, addr: &SocketAddr) -> Result<(), hyper::Error> {
        let service = hyper::service::make_service_fn(move |_| {
            let ep = self.ep.clone();
            async move {
                Ok::<_, Infallible>(hyper::service::service_fn({
                    move |req: hyper::Request<hyper::Body>| {
                        let ep = ep.clone();
                        async move {
                            let req = match Request::from_hyper(req) {
                                Ok(req) => req,
                                Err(err) => return Ok(err.as_response().into_hyper()),
                            };

                            let resp = match ep.call(req).await {
                                Ok(resp) => resp.into_hyper(),
                                Err(err) => err.as_response().into_hyper(),
                            };
                            Ok::<_, Infallible>(resp)
                        }
                    }
                }))
            }
        });

        hyper::Server::bind(addr).serve(service).await
    }
}
