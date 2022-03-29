use std::path::PathBuf;

use clap::Parser;
use poem::{listener::TcpListener, Server};
use poem_wasmhandler::wasmtime::{Caller, Extern, Trap};
use poem_wasmhandler::{WasmEndpointBuilder, WasmEndpointState};

#[derive(Parser)]
struct Options {
    /// Wasm file path
    file: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let options: Options = Options::parse();
    let wasm = std::fs::read(options.file).unwrap();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(
            WasmEndpointBuilder::new(wasm)
                .udf("env", "udf_add", |a: i32, b: i32| a + b)
                .udf(
                    "env",
                    "udf_touppercase",
                    |mut caller: Caller<WasmEndpointState>, buf: u32, buf_len: u32| {
                        let memory = match caller.get_export("memory") {
                            Some(Extern::Memory(memory)) => memory,
                            _ => return Err(Trap::new("memory not found")),
                        };
                        let memory = memory.data_mut(&mut caller);
                        let s = std::str::from_utf8_mut(
                            &mut memory[buf as usize..(buf + buf_len) as usize],
                        )
                        .map_err(|err| Trap::new(err.to_string()))?;
                        s.make_ascii_uppercase();
                        Ok(())
                    },
                )
                .build()
                .unwrap(),
        )
        .await
}
