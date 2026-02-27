use poem_mcpserver::{
    McpServer,
    protocol::{
        JSON_RPC_VERSION,
        resources::{ResourcesListRequest, ResourcesReadRequest},
        rpc::{Request, RequestId, Requests},
    },
};

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
