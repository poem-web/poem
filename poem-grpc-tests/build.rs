use std::io::Result;

fn main() -> Result<()> {
    poem_grpc_build::Config::new().compile(&["proto/test_harness.proto"], &["proto/"])?;

    // for `MethodType` tests, with client and server middlewares
    poem_grpc_build::Config::new()
        .client_middleware("crate::method_type_harness::ForwardMethodType")
        .server_middleware("crate::method_type_harness::EchoMethodType(\"x-server-middleware\")")
        .compile(&["proto/method_type_harness.proto"], &["proto/"])?;

    Ok(())
}
