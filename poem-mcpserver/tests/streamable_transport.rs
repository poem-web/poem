#![cfg(feature = "streamable-http")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use poem::{Endpoint, test::TestClient, web::sse::Event};
use poem_mcpserver::{
    McpServer,
    protocol::{
        rpc::RpcError,
        tool::{Tool, ToolsCallResponse},
    },
    streamable_http::{Config, endpoint_with_config},
    tool::Tools,
};
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_stream::StreamExt;

async fn initialize<E: Endpoint>(client: &TestClient<E>) -> String {
    let response = client
        .post("/")
        .header("Accept", "application/json, text/event-stream")
        .body_json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26", "capabilities": {},
                "clientInfo": { "name": "regression", "version": "1" }
            }
        }))
        .send()
        .await;
    response.assert_status_is_ok();
    response.0.headers()["Mcp-Session-Id"]
        .to_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn streamable_http_get_does_not_reroute_post_response() {
    let client = TestClient::new(endpoint_with_config(
        |_| McpServer::new(),
        Config {
            session_timeout: None,
        },
    ));
    let session_id = initialize(&client).await;
    let response = client
        .get("/")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await;
    let mut notifications = response.sse_stream();
    notifications.next().await.unwrap();

    let response = client
        .post("/")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .body_json(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        .send()
        .await;
    response.assert_status_is_ok();
    response
        .assert_json(json!([{"jsonrpc": "2.0", "id": 2, "result": {}}]))
        .await;
}

#[tokio::test]
async fn streamable_http_get_preserves_post_sse_response() {
    let client = TestClient::new(endpoint_with_config(
        |_| McpServer::new(),
        Config {
            session_timeout: None,
        },
    ));
    let session_id = initialize(&client).await;
    let response = client
        .get("/")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await;
    let mut notifications = response.sse_stream();
    notifications.next().await.unwrap();

    let response = client
        .post("/")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "text/event-stream, application/json")
        .body_json(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        .send()
        .await;
    response.assert_status_is_ok();
    let mut responses = response.sse_stream();
    let Event::Message { data, .. } = responses.next().await.unwrap() else {
        panic!("expected response event");
    };
    assert_eq!(
        serde_json::from_str::<Value>(&data).unwrap(),
        json!({
            "jsonrpc": "2.0", "id": 2, "result": {}
        })
    );
    assert!(responses.next().await.is_none());
}

#[derive(Clone)]
struct BlockingTools {
    started: Arc<Notify>,
    release: Arc<Notify>,
    completed: Arc<AtomicUsize>,
}

impl Tools for BlockingTools {
    fn instructions() -> &'static str {
        ""
    }
    fn list() -> Vec<Tool> {
        vec![]
    }
    async fn call(
        &mut self,
        _name: &str,
        _arguments: Value,
    ) -> Result<ToolsCallResponse, RpcError> {
        self.started.notify_one();
        self.release.notified().await;
        self.completed.fetch_add(1, Ordering::SeqCst);
        Ok(ToolsCallResponse {
            content: vec![],
            structured_content: None,
            is_error: false,
        })
    }
}

#[tokio::test]
async fn dropping_get_during_tool_call_keeps_post_response() {
    let tools = BlockingTools {
        started: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
        completed: Arc::new(AtomicUsize::new(0)),
    };
    let factory_tools = tools.clone();
    let client = TestClient::new(endpoint_with_config(
        move |_| McpServer::new().tools(factory_tools.clone()),
        Config {
            session_timeout: None,
        },
    ));
    let session_id = initialize(&client).await;
    let response = client
        .get("/")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await;
    let mut notifications = response.sse_stream();
    notifications.next().await.unwrap();

    let post = async {
        client
            .post("/")
            .header("Mcp-Session-Id", &session_id)
            .header("Accept", "application/json, text/event-stream")
            .body_json(&json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "side_effect", "arguments": {} }
            }))
            .send()
            .await
    };
    let disconnect = async {
        tools.started.notified().await;
        drop(notifications);
        tools.release.notify_one();
    };
    let (response, ()) = tokio::join!(post, disconnect);
    assert_eq!(tools.completed.load(Ordering::SeqCst), 1);
    response.assert_status_is_ok();
    response
        .assert_json(json!([{
            "jsonrpc": "2.0", "id": 2,
            "result": { "content": [], "isError": false }
        }]))
        .await;
}

#[tokio::test]
async fn legacy_sse_still_receives_post_responses() {
    let client = TestClient::new(endpoint_with_config(
        |_| McpServer::new(),
        Config {
            session_timeout: None,
        },
    ));
    let response = client.get("/").send().await;
    let mut notifications = response.sse_stream();
    let Event::Message { data: endpoint, .. } = notifications.next().await.unwrap() else {
        panic!("expected endpoint event");
    };
    client
        .post(format!("/{endpoint}"))
        .body_json(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        .send()
        .await
        .assert_status(poem::http::StatusCode::ACCEPTED);
    let Event::Message { data, .. } = notifications.next().await.unwrap() else {
        panic!("expected response event");
    };
    let response: Value = serde_json::from_str(&data).unwrap();
    assert_eq!(response["id"], 2);
    assert_eq!(response["result"], json!({}));
}
