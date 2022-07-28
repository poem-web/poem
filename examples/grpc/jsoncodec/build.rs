use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .codec("::poem_grpc::codec::JsonCodec")
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .compile(&["./proto/helloworld.proto"], &["./proto"])
}
