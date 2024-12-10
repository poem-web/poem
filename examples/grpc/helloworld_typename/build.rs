use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .enable_type_names()
        .compile(&["./proto/helloworld.proto"], &["./proto"])
}
