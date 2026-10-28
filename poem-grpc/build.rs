use std::io::Result;

fn main() -> Result<()> {
    poem_grpc_build::Config::new()
        .build_client(false)
        .internal()
        .file_descriptor_set_path("grpc-reflection.bin")
        .compile(
            &[
                // Canonical upstream paths, so these files can't clash with a
                // user's `health.proto` or `reflection.proto`.
                "proto/grpc/reflection/v1alpha/reflection.proto",
                "proto/grpc/reflection/v1/reflection.proto",
                "proto/grpc/health/v1/health.proto",
            ],
            &["proto/"],
        )?;

    // for test
    poem_grpc_build::Config::new()
        .internal()
        .compile(&["proto/test_harness.proto"], &["proto/"])?;

    // for `MethodType` tests, with client and server middlewares
    poem_grpc_build::Config::new()
        .internal()
        .client_middleware("crate::method_type_harness::ForwardMethodType")
        .server_middleware("crate::method_type_harness::EchoMethodType(\"x-server-middleware\")")
        .compile(&["proto/method_type_harness.proto"], &["proto/"])?;

    // example
    poem_grpc_build::Config::new()
        .internal()
        .compile(&["src/example_generated/routeguide.proto"], &[] as &[&str])?;

    Ok(())
}
