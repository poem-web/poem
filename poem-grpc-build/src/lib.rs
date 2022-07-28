//! Codegen module of `poem-grpc

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod client;
mod config;
mod server;
mod service_generator;
mod utils;

use std::path::Path;

pub use config::Config;

/// Compile .proto files into Rust files during a Cargo build with default
/// options.
pub fn compile_protos(
    protos: &[impl AsRef<Path>],
    includes: &[impl AsRef<Path>],
) -> std::io::Result<()> {
    Config::new().compile(protos, includes)
}
