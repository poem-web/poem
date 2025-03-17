//! Types for tool.

use std::{fmt::Display, future::Future};

use serde_json::Value;

use crate::protocol::{
    rpc::RpcError,
    tool::{Content, Tool as PTool, ToolsCallResponse},
};

/// Represents the result of a tool call.
pub trait IntoToolResponse {
    /// Consumes the object and converts it into a tool response.
    fn into_tool_response(self) -> ToolsCallResponse;
}

/// A Text response.
#[derive(Debug, Clone, Copy)]
pub struct Text<T>(pub T);

impl<T> IntoToolResponse for Text<T>
where
    T: Display,
{
    fn into_tool_response(self) -> ToolsCallResponse {
        ToolsCallResponse {
            content: vec![Content::Text {
                text: self.0.to_string(),
            }],
            is_error: false,
        }
    }
}

impl<T, E> IntoToolResponse for Result<T, E>
where
    T: IntoToolResponse,
    E: Display,
{
    fn into_tool_response(self) -> ToolsCallResponse {
        match self {
            Ok(value) => value.into_tool_response(),
            Err(error) => ToolsCallResponse {
                content: vec![Content::Text {
                    text: error.to_string(),
                }],
                is_error: true,
            },
        }
    }
}

/// Represents a tools collection.
pub trait Tools: Send + Sync {
    /// Returns the instructions for the tools.
    fn instructions() -> &'static str;

    /// Returns a list of tools.
    fn list() -> Vec<PTool>;

    /// Calls a tool.
    fn call(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> impl Future<Output = Result<ToolsCallResponse, RpcError>> + Send;
}

/// Empty tools collection.
#[derive(Debug, Clone, Copy)]
pub struct NoTools;

impl Tools for NoTools {
    #[inline]
    fn instructions() -> &'static str {
        ""
    }

    #[inline]
    fn list() -> Vec<PTool> {
        vec![]
    }

    #[inline]
    async fn call(&mut self, name: &str, _arguments: Value) -> Result<ToolsCallResponse, RpcError> {
        Err(RpcError::method_not_found(format!(
            "tool '{}' not found",
            name
        )))
    }
}
