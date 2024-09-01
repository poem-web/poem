use poem::{listener::TcpListener, Server};
use poem_grpc::{CompressionEncoding, Request, Response, RouteGrpc, Status};

poem_grpc::include_proto!("helloworld");

struct GreeterService;

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
    let route = RouteGrpc::new().add_service(
        GreeterServer::new(GreeterService)
            .send_compressed(CompressionEncoding::GZIP)
            .accept_compressed([
                CompressionEncoding::GZIP,
                CompressionEncoding::DEFLATE,
                CompressionEncoding::BROTLI,
                CompressionEncoding::ZSTD,
            ]),
    );
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(route)
        .await
}
