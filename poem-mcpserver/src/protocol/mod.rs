//! The protocol module contains all the necessary components to implement the MCP protocol.

pub mod initialize;
pub mod rpc;
pub mod tool;

/// The JSON-RPC version.
pub const JSON_RPC_VERSION: &str = "2.0";

/// The MCP protocol version.
pub const MCP_PROTOCOL_VERSION: time::Date = time::macros::date!(2024 - 11 - 05);
