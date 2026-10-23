use poem::{EndpointExt, Server, listener::TcpListener, middleware::Tracing};
use poem_grpc::{Request, Response, RouteGrpc, Status};

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
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(
            RouteGrpc::new()
                .add_service(GreeterServer::new(GreeterService))
                .with(Tracing),
        )
        .await
}
