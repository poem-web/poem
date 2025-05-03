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
#[derive(Debug, Serialize)]
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

/// A response to a resources/list request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResponse {
    /// Resources list.
    pub resources: Vec<Resource>,
}
