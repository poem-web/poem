use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .file_descriptor_set_path("helloworld.bin")
        .compile(&["./proto/helloworld.proto"], &["./proto"])
}
