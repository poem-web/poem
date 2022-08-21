mod middleware;

use poem::{listener::TcpListener, Server};
use poem_grpc::{Request, Response, RouteGrpc, Status};

poem_grpc::include_proto!("helloworld");

struct GreeterService;

#[poem::async_trait]
impl Greeter for GreeterService {
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
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(RouteGrpc::new().add_service(GreeterServer::new(GreeterService)))
        .await
}
