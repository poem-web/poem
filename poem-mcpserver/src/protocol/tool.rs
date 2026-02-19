//! Tool protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::content::Content;

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
    /// The output schema of the tool, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// The tool metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ToolMeta>,
}

/// Tool metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMeta {
    /// UI metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<ToolUi>,
}

/// Tool UI metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUi {
    /// UI resource URI.
    pub resource_uri: String,
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

/// A response to a tools/call request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallResponse {
    /// Response content.
    pub content: Vec<Content>,
    /// Structured content (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    /// Whether the response is an error.
    pub is_error: bool,
}
