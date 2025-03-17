//! MCP Server implementation for Poem

#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]

pub mod protocol;
mod server;
#[cfg(feature = "sse")]
pub mod sse;
pub mod stdio;
pub mod tool;
pub use poem_mcpserver_macros::Tools;
pub use server::McpServer;

#[doc(hidden)]
pub mod private {
    pub use serde_json;

    pub use crate::tool::IntoToolResponse;
}
