//! Types for prompts.

use std::future::Future;

use crate::{
    content::IntoContent,
    protocol::{
        content::Content,
        prompts::{Prompt, PromptGetResponse, PromptMessage, Role},
        rpc::RpcError,
    },
};

/// Represents a type that can be converted into a prompt response.
pub trait IntoPromptResponse {
    /// Consumes the object and converts it into a prompt response.
    fn into_prompt_response(self) -> PromptGetResponse;
}

impl IntoPromptResponse for PromptGetResponse {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        self
    }
}

impl IntoPromptResponse for PromptMessage {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: vec![self],
        }
    }
}

impl IntoPromptResponse for Vec<PromptMessage> {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: self,
        }
    }
}

impl IntoPromptResponse for String {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: vec![PromptMessage {
                role: Role::User,
                content: Content::Text { text: self },
            }],
        }
    }
}

impl IntoPromptResponse for &str {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: vec![PromptMessage {
                role: Role::User,
                content: Content::Text {
                    text: self.to_string(),
                },
            }],
        }
    }
}

impl<T> IntoPromptResponse for (Role, T)
where
    T: IntoContent,
{
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: vec![PromptMessage {
                role: self.0,
                content: self.1.into_content(),
            }],
        }
    }
}

/// A builder for creating prompt responses with multiple messages.
#[derive(Debug, Default)]
pub struct PromptMessages {
    messages: Vec<PromptMessage>,
}

impl PromptMessages {
    /// Creates a new empty prompt messages builder.
    #[inline]
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Adds a user message to the prompt.
    #[inline]
    pub fn user(mut self, content: impl IntoContent) -> Self {
        self.messages.push(PromptMessage {
            role: Role::User,
            content: content.into_content(),
        });
        self
    }

    /// Adds an assistant message to the prompt.
    #[inline]
    pub fn assistant(mut self, content: impl IntoContent) -> Self {
        self.messages.push(PromptMessage {
            role: Role::Assistant,
            content: content.into_content(),
        });
        self
    }

    /// Adds a message with a specific role to the prompt.
    #[inline]
    pub fn message(mut self, role: Role, content: impl IntoContent) -> Self {
        self.messages.push(PromptMessage {
            role,
            content: content.into_content(),
        });
        self
    }
}

impl IntoPromptResponse for PromptMessages {
    #[inline]
    fn into_prompt_response(self) -> PromptGetResponse {
        PromptGetResponse {
            description: "",
            messages: self.messages,
        }
    }
}

/// Represents a prompts collection.
pub trait Prompts {
    /// Returns a list of prompts.
    fn list() -> Vec<Prompt>;

    /// Gets a prompt by name with the given arguments.
    fn get(
        &self,
        name: &str,
        arguments: std::collections::HashMap<String, String>,
    ) -> impl Future<Output = Result<PromptGetResponse, RpcError>> + Send;
}

/// Empty prompts collection.
#[derive(Debug, Clone, Copy)]
pub struct NoPrompts;

impl Prompts for NoPrompts {
    #[inline]
    fn list() -> Vec<Prompt> {
        vec![]
    }

    #[inline]
    async fn get(
        &self,
        name: &str,
        _arguments: std::collections::HashMap<String, String>,
    ) -> Result<PromptGetResponse, RpcError> {
        Err(RpcError::method_not_found(format!(
            "prompt '{name}' not found"
        )))
    }
}
