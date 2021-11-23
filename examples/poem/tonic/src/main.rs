use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};
use poem::{endpoint::TowerCompatExt, listener::TcpListener, Route, Server};
use tonic::{transport::NamedService, Request, Response, Status};

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
        println!("Got a request from {:?}", request.remote_addr());

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

    let app = Route::new().nest_no_strip(
        format!("/{}", GreeterServer::<MyGreeter>::NAME),
        GreeterServer::new(MyGreeter).compat(),
    );
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
