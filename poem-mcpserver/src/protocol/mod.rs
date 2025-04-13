//! The protocol module contains all the necessary components to implement the
//! MCP protocol.

pub mod content;
pub mod initialize;
pub mod prompts;
pub mod resources;
pub mod rpc;
pub mod tool;

/// The JSON-RPC version.
pub const JSON_RPC_VERSION: &str = "2.0";

/// The MCP protocol version.
pub const MCP_PROTOCOL_VERSION: time::Date = time::macros::date!(2025 - 03 - 26);
