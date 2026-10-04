use std::{
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};

use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};
use poem::{endpoint::TowerCompatExt, listener::TcpListener, Route, Server};
use tonic::{Request, Response, Status};
use tower::{buffer::Buffer, util::MapResponse};

// Tonic's streaming body is Send but not Sync. The compatibility endpoint
// requires both, so guard access without collecting the body or discarding gRPC
// trailers.
struct SyncBody(Mutex<tonic::body::Body>);

impl http_body::Body for SyncBody {
    type Data = <tonic::body::Body as http_body::Body>::Data;
    type Error = tonic::Status;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(self.get_mut().0.get_mut().expect("body mutex poisoned")).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.0.lock().expect("body mutex poisoned").is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.0.lock().expect("body mutex poisoned").size_hint()
    }
}

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let service = Buffer::new(
        MapResponse::new(
            tonic::service::Routes::new(GreeterServer::new(MyGreeter)),
            |response: poem::http::Response<tonic::body::Body>| {
                response.map(|body| SyncBody(Mutex::new(body)))
            },
        ),
        1024,
    );
    let app = Route::new().nest_no_strip("/", service.compat());

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
