//! MCP Server implementation for Poem

#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]

pub mod content;
pub mod protocol;
mod server;
pub mod stdio;
#[cfg(feature = "streamable-http")]
#[cfg_attr(docsrs, doc(cfg(feature = "streamable-http")))]
pub mod streamable_http;
pub mod tool;
pub use poem_mcpserver_macros::Tools;
pub use server::McpServer;

#[doc(hidden)]
pub mod private {
    pub use serde_json;

    pub use crate::tool::IntoToolResponse;
}
