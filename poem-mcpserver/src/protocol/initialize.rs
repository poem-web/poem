//! Initialize request and response.

use serde::{Deserialize, Serialize};
use time::Date;

/// The client capabilities.
#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {}

/// The client information.
#[derive(Debug, Deserialize)]
pub struct ClientInfo {
    /// The client name.
    pub name: String,
    /// The client version.
    pub version: String,
}

/// An initialize request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    /// The protocol version.
    #[serde(with = "protocol_version_serde")]
    pub protocol_version: Date,
    /// The client capabilities.
    pub capabilities: ClientCapabilities,
    /// The client information.
    pub client_info: ClientInfo,
}

/// Prompts capability.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Indicates whether the server will emit notifications when the list of
    /// available prompts changes.
    pub list_changed: bool,
}

/// Resources capability.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Indicates whether the server will emit notifications when the list of
    /// available prompts changes.
    pub list_changed: bool,
    /// Whether the client can subscribe to be notified of changes to individual
    /// resources.
    pub subscribe: bool,
}

/// Tools capability.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Indicates whether the server will emit notifications when the list of
    /// available prompts changes.
    pub list_changed: bool,
}

/// The server capabilities.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Prompts capability.
    pub prompts: PromptsCapability,
    /// Resources capability.
    pub resources: ResourcesCapability,
    /// Tools capability.
    pub tools: ToolsCapability,
}

/// The server information.
#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    /// The server name.
    pub name: String,
    /// The server version.
    pub version: String,
}

/// An initialize response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResponse {
    /// The protocol version.
    #[serde(with = "protocol_version_serde")]
    pub protocol_version: Date,
    /// The server capabilities.
    pub capabilities: ServerCapabilities,
    /// The server information.
    pub server_info: ServerInfo,
    /// The server instructions.
    pub instructions: Option<String>,
}

mod protocol_version_serde {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use time::{Date, format_description::BorrowedFormatItem};

    const DESC: &[BorrowedFormatItem] = time::macros::format_description!("[year]-[month]-[day]");

    pub(super) fn serialize<S>(date: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format(DESC).unwrap())
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Date::parse(&s, DESC).map_err(|err| D::Error::custom(err.to_string()))
    }
}
