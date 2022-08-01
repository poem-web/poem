use std::{env, io::Result, path::PathBuf};

fn main() -> Result<()> {
    let reflection_descriptor =
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("reflection_v1alpha1.bin");

    poem_grpc_build::Config::new()
        .build_client(false)
        .internal()
        .file_descriptor_set_path(&reflection_descriptor)
        .compile(&["proto/reflection.proto"], &["proto/"])
}
