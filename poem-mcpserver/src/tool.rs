//! Types for tool.

use std::{fmt::Display, future::Future};

use schemars::{JsonSchema, Schema};
use serde::Serialize;
use serde_json::Value;

use crate::{
    content::IntoContents,
    protocol::{
        content::Content,
        rpc::RpcError,
        tool::{Tool as PTool, ToolsCallResponse},
    },
};

/// Represents the result of a tool call.
pub trait IntoToolResponse {
    /// Returns the output schema of the tool response, if any.
    fn output_schema() -> Option<Schema>;

    /// Consumes the object and converts it into a tool response.
    fn into_tool_response(self) -> ToolsCallResponse;
}

impl IntoToolResponse for () {
    fn output_schema() -> Option<Schema> {
        None
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        ToolsCallResponse {
            content: vec![],
            structured_content: None,
            is_error: false,
        }
    }
}

impl<E> IntoToolResponse for Result<(), E>
where
    E: Display,
{
    fn output_schema() -> Option<Schema> {
        None
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        match self {
            Ok(_) => ToolsCallResponse {
                content: vec![],
                structured_content: None,
                is_error: false,
            },
            Err(error) => ToolsCallResponse {
                content: vec![Content::Text {
                    text: error.to_string(),
                }],
                structured_content: None,
                is_error: true,
            },
        }
    }
}

impl<T> IntoToolResponse for T
where
    T: IntoContents,
{
    fn output_schema() -> Option<Schema> {
        None
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        ToolsCallResponse {
            content: self.into_contents(),
            structured_content: None,
            is_error: false,
        }
    }
}

impl<T, E> IntoToolResponse for Result<T, E>
where
    T: IntoContents,
    E: Display,
{
    fn output_schema() -> Option<Schema> {
        None
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        match self {
            Ok(value) => ToolsCallResponse {
                content: value.into_contents(),
                structured_content: None,
                is_error: false,
            },
            Err(error) => ToolsCallResponse {
                content: vec![Content::Text {
                    text: error.to_string(),
                }],
                structured_content: None,
                is_error: true,
            },
        }
    }
}

/// A Structured content.
#[derive(Debug, Clone, Copy)]
pub struct StructuredContent<T>(pub T);

impl<T> IntoToolResponse for StructuredContent<T>
where
    T: Serialize + JsonSchema,
{
    fn output_schema() -> Option<Schema> {
        let schema = schemars::SchemaGenerator::default().into_root_schema_for::<T>();
        if let Ok(value) = serde_json::to_value(&schema) {
            if value.get("type") == Some(&serde_json::Value::String("array".to_string())) {
                panic!(
                    "Tool return type must be an object, but found array. Please wrap the return value in a struct."
                );
            }
        }
        Some(schema)
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        ToolsCallResponse {
            content: vec![Content::Text {
                text: serde_json::to_string(&self.0).unwrap_or_default(),
            }],
            structured_content: Some(serde_json::to_value(&self.0).unwrap_or_default()),
            is_error: false,
        }
    }
}

impl<T, E> IntoToolResponse for Result<StructuredContent<T>, E>
where
    T: Serialize + JsonSchema,
    E: Display,
{
    fn output_schema() -> Option<Schema> {
        let schema = schemars::SchemaGenerator::default().into_root_schema_for::<T>();
        if let Ok(value) = serde_json::to_value(&schema) {
            if value.get("type") == Some(&serde_json::Value::String("array".to_string())) {
                panic!(
                    "Tool return type must be an object, but found array. Please wrap the return value in a struct."
                );
            }
        }
        Some(schema)
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        match self {
            Ok(value) => ToolsCallResponse {
                content: vec![Content::Text {
                    text: serde_json::to_string(&value.0).unwrap_or_default(),
                }],
                structured_content: Some(serde_json::to_value(&value.0).unwrap_or_default()),
                is_error: false,
            },
            Err(error) => ToolsCallResponse {
                content: vec![Content::Text {
                    text: error.to_string(),
                }],
                structured_content: None,
                is_error: true,
            },
        }
    }
}

// impl IntoToolResponse for Json

/// Represents a tools collection.
pub trait Tools {
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
            "tool '{name}' not found"
        )))
    }
}
