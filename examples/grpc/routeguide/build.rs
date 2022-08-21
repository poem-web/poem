use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .type_attribute("routeguide.Point", "#[derive(Hash, Eq)]")
        .compile(&["./proto/routeguide.proto"], &["./proto"])
}
