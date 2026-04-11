use poem_mcpserver::{
    McpServer,
    protocol::{
        JSON_RPC_VERSION,
        resources::{ResourcesListRequest, ResourcesReadRequest},
        rpc::{Request, RequestId, Requests},
    },
    resources::Resources,
};

struct DynamicResources;

impl Resources for DynamicResources {
    async fn list(
        &self,
        _request: poem_mcpserver::protocol::resources::ResourcesListRequest,
    ) -> Result<
        poem_mcpserver::protocol::resources::ResourcesListResponse,
        poem_mcpserver::protocol::rpc::RpcError,
    > {
        Ok(poem_mcpserver::protocol::resources::ResourcesListResponse {
            resources: vec![poem_mcpserver::protocol::resources::Resource {
                uri: "dyn://hello".to_string(),
                name: "Dynamic Hello".to_string(),
                description: "Dynamic resource".to_string(),
                mime_type: "text/plain".to_string(),
            }],
        })
    }

    async fn templates(
        &self,
        _request: poem_mcpserver::protocol::resources::ResourcesTemplatesListRequest,
    ) -> Result<
        poem_mcpserver::protocol::resources::ResourcesTemplatesListResponse,
        poem_mcpserver::protocol::rpc::RpcError,
    > {
        Ok(
            poem_mcpserver::protocol::resources::ResourcesTemplatesListResponse {
                resource_templates: vec![],
            },
        )
    }

    async fn read(
        &self,
        request: poem_mcpserver::protocol::resources::ResourcesReadRequest,
    ) -> Result<
        poem_mcpserver::protocol::resources::ResourcesReadResponse,
        poem_mcpserver::protocol::rpc::RpcError,
    > {
        Ok(poem_mcpserver::protocol::resources::ResourcesReadResponse {
            contents: vec![poem_mcpserver::protocol::resources::ResourceContent {
                uri: request.uri,
                mime_type: "text/plain".to_string(),
                text: Some("hello".to_string()),
                blob: None,
            }],
        })
    }
}

#[tokio::test]
async fn resources_list_and_read() {
    let mut server = McpServer::new().ui_resource(
        "ui://apps/demo",
        "Demo App",
        "Demo UI resource",
        "text/html",
        "<html>demo</html>",
    );

    let list_resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ResourcesList {
                params: ResourcesListRequest { cursor: None },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&list_resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": [
                    {
                        "uri": "ui://apps/demo",
                        "name": "Demo App",
                        "description": "Demo UI resource",
                        "mimeType": "text/html"
                    }
                ]
            }
        })
    );

    let read_resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(2)),
            body: Requests::ResourcesRead {
                params: ResourcesReadRequest {
                    uri: "ui://apps/demo".to_string(),
                },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&read_resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "contents": [
                    {
                        "uri": "ui://apps/demo",
                        "mimeType": "text/html",
                        "text": "<html>demo</html>"
                    }
                ]
            }
        })
    );
}

#[tokio::test]
async fn resources_templates_list() {
    let mut server = McpServer::new();

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ResourcesTemplatesList {
                params: Default::default(),
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resourceTemplates": []
            }
        })
    );
}

#[tokio::test]
async fn dynamic_resources_list_and_read() {
    let mut server = McpServer::new().resources(DynamicResources);

    let list_resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::ResourcesList {
                params: ResourcesListRequest { cursor: None },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&list_resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": [
                    {
                        "uri": "dyn://hello",
                        "name": "Dynamic Hello",
                        "description": "Dynamic resource",
                        "mimeType": "text/plain"
                    }
                ]
            }
        })
    );

    let read_resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(2)),
            body: Requests::ResourcesRead {
                params: ResourcesReadRequest {
                    uri: "dyn://hello".to_string(),
                },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&read_resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "contents": [
                    {
                        "uri": "dyn://hello",
                        "mimeType": "text/plain",
                        "text": "hello"
                    }
                ]
            }
        })
    );
}
