//! Tool protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A request to list tools.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListRequest {
    /// The cursor to continue listing tools.
    pub cursor: Option<String>,
}

/// Tool information.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool.
    pub name: &'static str,
    /// The description of the tool.
    pub description: &'static str,
    /// The input schema of the tool.
    pub input_schema: Value,
}

/// A response to a tools/list request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResponse {
    /// Tools list.
    pub tools: Vec<Tool>,
}

/// A request to call a tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallRequest {
    /// The name of the tool.
    pub name: String,
    #[serde(default)]
    /// The arguments passed to the tool.
    pub arguments: Value,
}

/// A content that can be sent to the client.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Content {
    /// A text content.
    Text {
        /// The text content.
        text: String,
    },
}

/// A response to a tools/call request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallResponse {
    /// Response content.
    pub content: Vec<Content>,
    /// Whether the response is an error.
    pub is_error: bool,
}
