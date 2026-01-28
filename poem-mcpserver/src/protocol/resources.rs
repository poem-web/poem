//! Resource protocol.

use serde::{Deserialize, Serialize};

/// A request to list resources.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListRequest {
    /// The cursor to continue listing tools.
    pub cursor: Option<String>,
}

/// Resource information.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// The uri of the resource.
    pub uri: String,
    /// The name of the tool.
    pub name: String,
    /// The description of the tool.
    pub description: String,
    /// The mime type of the resource.
    pub mime_type: String,
}

/// A request to read resources.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesReadRequest {
    /// The uri of the resource.
    pub uri: String,
}

/// Resource content.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    /// The uri of the resource.
    pub uri: String,
    /// The mime type of the resource.
    pub mime_type: String,
    /// Text content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded binary content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// A response to a resources/list request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResponse {
    /// Resources list.
    pub resources: Vec<Resource>,
}

/// A response to a resources/read request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesReadResponse {
    /// Resources contents.
    pub contents: Vec<ResourceContent>,
}
