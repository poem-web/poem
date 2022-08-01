use poem::{listener::TcpListener, middleware::Tracing, EndpointExt, Server};
use poem_grpc::{Reflection, Request, Response, Route, Status};

poem_grpc::include_proto!("helloworld");
const FILE_DESCRIPTOR_SET: &[u8] = poem_grpc::include_file_descriptor_set!("helloworld.bin");

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
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(
            Route::new()
                .add_service(
                    Reflection::new()
                        .add_file_descriptor_set(FILE_DESCRIPTOR_SET)
                        .build(),
                )
                .add_service(GreeterServer::new(GreeterService))
                .with(Tracing),
        )
        .await
}
