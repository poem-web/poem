#![cfg(feature = "streamable-http")]

use poem::{http::header::CONTENT_TYPE, test::TestClient};
use poem_mcpserver::{McpServer, streamable_http};
use serde_json::{Value, json};

#[tokio::test]
async fn request_scoped_resources_remain_isolated_between_sessions() {
    let client = TestClient::new(streamable_http::endpoint(|request| {
        let tenant = request.headers()["x-tenant"].to_str().unwrap();
        McpServer::new().ui_resource(
            "ui://account",
            "Account",
            "Private account UI",
            "text/html",
            format!("private account information for {tenant}"),
        )
    }));

    for tenant in ["alice", "bob"] {
        let response = client
            .post("/")
            .header("x-tenant", tenant)
            .header("Accept", "application/json, text/event-stream")
            .body_json(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26", "capabilities": {},
                    "clientInfo": { "name": tenant, "version": "1" }
                }
            }))
            .send()
            .await;
        response.assert_status_is_ok();
        let session_id = response.0.headers()["Mcp-Session-Id"]
            .to_str()
            .unwrap()
            .to_owned();

        let response = client
            .post("/")
            .header("x-tenant", tenant)
            .header("Mcp-Session-Id", session_id)
            .header("Accept", "application/json, text/event-stream")
            .header(CONTENT_TYPE, "application/json")
            .body_json(&json!({
                "jsonrpc": "2.0", "id": 2, "method": "resources/read",
                "params": { "uri": "ui://account" }
            }))
            .send()
            .await;
        response.assert_status_is_ok();
        let body: Value = response.0.into_body().into_json().await.unwrap();
        assert_eq!(
            body[0]["result"]["contents"][0]["text"],
            format!("private account information for {tenant}")
        );
    }
}
