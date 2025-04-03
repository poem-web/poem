//! JSON-RPC protocol types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::{
    initialize::InitializeRequest,
    tool::{ToolsCallRequest, ToolsListRequest},
};

/// A JSON-RPC request id.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// A numeric request id.
    Int(i64),
    /// A string request id.
    String(String),
}

/// A JSON-RPC request body.
#[derive(Debug, Deserialize)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum Requests {
    /// Ping.
    Ping,
    /// Initialize.
    Initialize {
        /// Initialize request parameters.
        params: InitializeRequest,
    },
    /// Initialized notification.
    #[serde(rename = "notifications/initialized")]
    Initialized,
    /// Tools list.
    #[serde(rename = "tools/list")]
    ToolsList {
        /// Tools list request parameters.
        #[serde(default)]
        params: ToolsListRequest,
    },
    /// Call a tool.
    #[serde(rename = "tools/call")]
    ToolsCall {
        /// Tool call request parameters.
        params: ToolsCallRequest,
    },
}

/// A JSON-RPC request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// The JSON-RPC version.
    pub jsonrpc: String,
    /// The request id.
    pub id: Option<RequestId>,
    /// The request body.
    #[serde(flatten)]
    pub body: Requests,
}

/// A JSON-RPC response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T = ()> {
    /// The JSON-RPC version.
    pub jsonrpc: String,
    /// The request id.
    pub id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The response result.
    pub result: Option<T>,
    /// The response error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl<T> Response<T>
where
    T: Serialize,
{
    /// Convert the response body to `serde_json::Value`.
    #[inline]
    pub fn map_result_to_value(self) -> Response<Value> {
        Response {
            jsonrpc: self.jsonrpc,
            id: self.id,
            result: self
                .result
                .map(|v| serde_json::to_value(v).expect("serialize result")),
            error: self.error,
        }
    }
}

const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;

/// A JSON-RPC error.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcError<E = ()> {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<E>,
}

impl<E> RpcError<E> {
    /// Create a new JSON-RPC error with the given code and message.
    #[inline]
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        RpcError {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Attach data to the JSON-RPC error.
    #[inline]
    pub fn with_data<Q>(self, data: Q) -> RpcError<Q> {
        RpcError {
            code: self.code,
            message: self.message,
            data: Some(data),
        }
    }

    /// Create a JSON-RPC error with code `PARSE_ERROR(-32700)` and the given
    /// message.
    #[inline]
    pub fn parse_error(message: impl Into<String>) -> Self {
        RpcError::new(PARSE_ERROR, message)
    }

    /// Create a JSON-RPC error with code `INVALID_REQUEST(-32600)` and the
    /// given message.
    #[inline]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        RpcError::new(INVALID_REQUEST, message)
    }

    /// Create a JSON-RPC error with code `METHOD_NOT_FOUND(-32601)` and the
    /// given message.
    #[inline]
    pub fn method_not_found(message: impl Into<String>) -> Self {
        RpcError::new(METHOD_NOT_FOUND, message)
    }

    /// Create a JSON-RPC error with code `INVALID_PARAMS(-32602)` and the given
    /// message.
    #[inline]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        RpcError::new(INVALID_PARAMS, message)
    }

    /// Create a JSON-RPC error with code `INTERNAL_ERROR(-32603)` and the given
    /// message.
    #[inline]
    pub fn internal_error(message: impl Into<String>) -> Self {
        RpcError::new(INTERNAL_ERROR, message)
    }
}
