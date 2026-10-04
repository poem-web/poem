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

fn is_nonstandard_uint_format(format: &str) -> bool {
    matches!(
        format,
        "uint" | "uint8" | "uint16" | "uint32" | "uint64" | "uint128"
    )
}

fn normalize_schema_value_inner(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if matches!(object.get("format"), Some(Value::String(format)) if is_nonstandard_uint_format(format))
            {
                object.remove("format");
            }

            for value in object.values_mut() {
                normalize_schema_value_inner(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                normalize_schema_value_inner(value);
            }
        }
        _ => {}
    }
}

#[doc(hidden)]
pub fn normalize_schema_value(mut value: Value) -> Value {
    normalize_schema_value_inner(&mut value);
    value
}

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

/// A structured tool result serialized as a JSON object.
///
/// MCP 2025-06-18 requires structured content to be an object. Wrap arrays and
/// scalar values in a struct, or use [`crate::content::Json`] for JSON text
/// without structured content. Nested arrays in an object are supported.
///
/// Only schemas with an explicit object root are advertised in `tools/list`.
/// Other schemas are omitted rather than panicking. A non-object result or a
/// serialization failure produces a tool error (`isError: true`) without
/// `structuredContent`. Object results are still supported when their schema
/// does not have an explicit object root (for example, a composed enum schema).
#[derive(Debug, Clone, Copy)]
pub struct StructuredContent<T>(pub T);

impl<T> IntoToolResponse for StructuredContent<T>
where
    T: Serialize + JsonSchema,
{
    fn output_schema() -> Option<Schema> {
        let schema = schemars::SchemaGenerator::default().into_root_schema_for::<T>();
        // MCP 2025-06-18 only permits an object output schema. An unsupported
        // return type must not prevent clients from discovering the other
        // tools.
        (schema.get("type").and_then(Value::as_str) == Some("object")).then_some(schema)
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        match serde_json::to_value(&self.0) {
            Ok(value) if value.is_object() => ToolsCallResponse {
                content: vec![Content::Text {
                    text: value.to_string(),
                }],
                structured_content: Some(value),
                is_error: false,
            },
            Ok(_) => structured_content_error(
                "Structured tool output must be a JSON object. Please wrap the return value in a struct."
                    .to_string(),
            ),
            Err(error) => structured_content_error(format!(
                "Failed to serialize structured tool output: {error}"
            )),
        }
    }
}

fn structured_content_error(text: String) -> ToolsCallResponse {
    ToolsCallResponse {
        content: vec![Content::Text { text }],
        structured_content: None,
        is_error: true,
    }
}

impl<T, E> IntoToolResponse for Result<StructuredContent<T>, E>
where
    T: Serialize + JsonSchema,
    E: Display,
{
    fn output_schema() -> Option<Schema> {
        StructuredContent::<T>::output_schema()
    }

    fn into_tool_response(self) -> ToolsCallResponse {
        match self {
            Ok(value) => value.into_tool_response(),
            Err(error) => structured_content_error(error.to_string()),
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

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde::{Serialize, Serializer};
    use serde_json::json;

    use super::{IntoToolResponse, StructuredContent, normalize_schema_value};

    #[test]
    fn strips_nonstandard_unsigned_integer_formats() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "format": "uint32",
                    "minimum": 0
                },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "format": "uint"
                    }
                },
                "signed": {
                    "type": "integer",
                    "format": "int32"
                }
            }
        });

        let normalized = normalize_schema_value(schema);

        assert_eq!(normalized["properties"]["count"]["type"], json!("integer"));
        assert_eq!(normalized["properties"]["count"]["minimum"], json!(0));
        assert!(normalized["properties"]["count"].get("format").is_none());
        assert!(
            normalized["properties"]["items"]["items"]
                .get("format")
                .is_none()
        );
        assert_eq!(normalized["properties"]["signed"]["format"], json!("int32"));
    }

    fn assert_non_object<T: Serialize + JsonSchema + Clone>(value: T) {
        assert!(StructuredContent::<T>::output_schema().is_none());
        assert!(<Result<StructuredContent<T>, &str>>::output_schema().is_none());
        for response in [
            StructuredContent(value.clone()).into_tool_response(),
            Ok::<_, &str>(StructuredContent(value)).into_tool_response(),
        ] {
            assert_eq!(
                serde_json::to_value(response).unwrap(),
                json!({
                    "content": [{
                        "type": "text",
                        "text": "Structured tool output must be a JSON object. Please wrap the return value in a struct."
                    }],
                    "isError": true,
                })
            );
        }
    }

    #[test]
    fn non_object_structured_results_are_tool_errors() {
        assert_non_object(Vec::<String>::new());
        assert_non_object(vec![vec![1, 2]]);
        assert_non_object([1, 2]);
        assert_non_object((1, "two"));
        assert_non_object("text");
        assert_non_object(42);
        assert_non_object(true);
        assert_non_object(());
        assert_non_object(Option::<String>::None);
        assert_non_object(json!([1, 2]));
    }

    #[test]
    fn object_results_with_composed_or_unconstrained_schemas_are_supported() {
        #[derive(JsonSchema, Serialize)]
        #[serde(untagged)]
        enum ObjectResult {
            Name { name: String },
            Items { items: Vec<String> },
        }

        assert!(StructuredContent::<ObjectResult>::output_schema().is_none());
        assert!(StructuredContent::<serde_json::Value>::output_schema().is_none());
        for value in [
            ObjectResult::Name {
                name: "test".to_string(),
            },
            ObjectResult::Items {
                items: vec!["a".to_string()],
            },
        ] {
            let expected = serde_json::to_value(&value).unwrap();
            let response = StructuredContent(value).into_tool_response();
            assert!(!response.is_error);
            assert_eq!(response.structured_content, Some(expected));
        }
        let value = json!({"items": [[1, 2], [3, 4]]});
        let response = StructuredContent(value.clone()).into_tool_response();
        assert!(!response.is_error);
        assert_eq!(response.structured_content, Some(value));
    }

    #[test]
    fn object_schema_does_not_allow_a_non_object_serialized_result() {
        #[derive(JsonSchema, Serialize)]
        #[serde(transparent)]
        #[schemars(with = "std::collections::BTreeMap<String, String>")]
        struct MisleadingSchema(Vec<String>);

        assert!(StructuredContent::<MisleadingSchema>::output_schema().is_some());
        let response = StructuredContent(MisleadingSchema(vec![])).into_tool_response();
        assert!(response.is_error);
        assert!(response.structured_content.is_none());
    }

    #[test]
    fn serialization_failures_are_tool_errors() {
        #[derive(Clone, JsonSchema)]
        struct SerializationFailure {}

        impl Serialize for SerializationFailure {
            fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("cannot serialize result"))
            }
        }

        assert!(StructuredContent::<SerializationFailure>::output_schema().is_some());
        for response in [
            StructuredContent(SerializationFailure {}).into_tool_response(),
            Ok::<_, &str>(StructuredContent(SerializationFailure {})).into_tool_response(),
        ] {
            assert_eq!(
                serde_json::to_value(response).unwrap(),
                json!({
                    "content": [{
                        "type": "text",
                        "text": "Failed to serialize structured tool output: cannot serialize result"
                    }],
                    "isError": true,
                })
            );
        }
    }
}
