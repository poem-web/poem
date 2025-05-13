use poem_mcpserver::{
    McpServer, Tools,
    content::Text,
    protocol::{
        JSON_RPC_VERSION,
        rpc::{Request, RequestId, Requests},
        tool::{ToolsCallRequest, ToolsListRequest},
    },
};

struct TestTools {
    value: i32,
}

impl TestTools {
    fn new() -> Self {
        Self { value: 0 }
    }
}

#[Tools]
impl TestTools {
    /// Add a value to the current value.
    async fn add_value(&mut self, value: i32) -> Text<i32> {
        self.value += value;
        Text(self.value)
    }

    /// Get the current value.
    async fn get_value(&self) -> Text<i32> {
        Text(self.value)
    }
}

#[tokio::test]
async fn call_tool() {
    let tools = TestTools::new();
    let mut server = McpServer::new().tools(tools);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ToolsCall {
                params: ToolsCallRequest {
                    name: "add_value".to_string(),
                    arguments: serde_json::json!({
                        "value": 10,
                    }),
                },
            },
        })
        .await;
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "10"}],
                "isError": false,
            },
        })
    );

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(5)),
            body: Requests::ToolsCall {
                params: ToolsCallRequest {
                    name: "add_value".to_string(),
                    arguments: serde_json::json!({
                        "value": 30,
                    }),
                },
            },
        })
        .await;
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "result": {
                "content": [{"type": "text", "text": "40"}],
                "isError": false,
            },
        })
    );

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(7)),
            body: Requests::ToolsCall {
                params: ToolsCallRequest {
                    name: "get_value".to_string(),
                    arguments: serde_json::json!({}),
                },
            },
        })
        .await;
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {
                "content": [{"type": "text", "text": "40"}],
                "isError": false,
            },
        })
    );
}

#[tokio::test]
async fn tool_list() {
    let tools = TestTools::new();
    let mut server = McpServer::new().tools(tools);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ToolsList {
                params: ToolsListRequest { cursor: None },
            },
        })
        .await;
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "add_value",
                        "description": "Add a value to the current value.",
                        "inputSchema": {
                            "$schema": "http://json-schema.org/draft-07/schema#",
                            "type": "object",
                            "properties": {
                                "value": {
                                    "format": "int32",
                                    "type": "integer",
                                },
                            },
                            "required": ["value"],
                            "title": "add_value_Request",
                        },
                    },
                    {
                        "name": "get_value",
                        "description": "Get the current value.",
                        "inputSchema": {
                            "$schema": "http://json-schema.org/draft-07/schema#",
                            "type": "object",
                            "properties": {},
                            "title": "get_value_Request",
                        },
                    },
                ],
            },
        })
    );
}

#[tokio::test]
async fn disable_tools() {
    let tools = TestTools::new();
    let mut server = McpServer::new().tools(tools).disable_tools(["get_value"]);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ToolsList {
                params: ToolsListRequest { cursor: None },
            },
        })
        .await;
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "add_value",
                        "description": "Add a value to the current value.",
                        "inputSchema": {
                            "$schema": "http://json-schema.org/draft-07/schema#",
                            "type": "object",
                            "properties": {
                                "value": {
                                    "format": "int32",
                                    "type": "integer",
                                },
                            },
                            "required": ["value"],
                            "title": "add_value_Request",
                        },
                    },
                ],
            },
        })
    );
}
