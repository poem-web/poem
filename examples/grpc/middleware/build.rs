use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .client_middleware("crate::middleware::ClientMiddleware")
        .server_middleware("crate::middleware::ServerMiddleware")
        .compile(&["./proto/helloworld.proto"], &["./proto"])
}
