use std::io::Result;

fn main() -> Result<()> {
    poem_grpc_build::Config::new()
        .build_client(false)
        .internal()
        .file_descriptor_set_path("grpc-reflection.bin")
        .compile(
            &["proto/reflection.proto", "proto/health.proto"],
            &["proto/"],
        )?;

    // for test
    poem_grpc_build::Config::new()
        .internal()
        .compile(&["proto/test_harness.proto"], &["proto/"])
}
