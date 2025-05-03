//! Prompt protocol.

use serde::{Deserialize, Serialize};

use crate::{content::IntoContent, protocol::content::Content};

/// A request to list prompts.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListRequest {
    /// The cursor to continue listing tools.
    pub cursor: Option<String>,
}

/// Prompt argument.
#[derive(Debug, Serialize)]
pub struct PromptArgument {
    /// The name of the argument.
    pub name: &'static str,
    /// The description of the argument.
    pub description: &'static str,
    /// Whether the argument is required.
    pub required: bool,
}

/// Prompt information.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// The name of the tool.
    pub name: &'static str,
    /// The description of the tool.
    pub description: &'static str,
    /// The input schema of the tool.
    pub arguments: &'static [PromptArgument],
}

/// A response to a prompts/list request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListResponse {
    /// Prompts list.
    pub prompts: Vec<Prompt>,
}

/// A role type to indicate the speaker.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    /// User
    User,
    /// Assistant
    Assistant,
}

/// A prompt message.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// The role type.
    pub role: Role,
    /// The content of the message.
    pub content: Content,
}

impl PromptMessage {
    /// Creates a new prompt message.
    #[inline]
    pub fn new(role: Role, content: impl IntoContent) -> Self {
        Self {
            role,
            content: content.into_content(),
        }
    }
}

/// A response to a prompts/get request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptGetResponse {
    /// Prompt description.
    pub description: &'static str,
    /// Whether the response is an error.
    pub messages: Vec<PromptMessage>,
}
