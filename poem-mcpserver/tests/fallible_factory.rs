#![cfg(feature = "streamable-http")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use poem::{
    Endpoint, EndpointExt, IntoEndpoint, Request, Response,
    http::{Method, StatusCode},
    test::{TestClient, TestResponse},
    web::sse::Event,
};
use poem_mcpserver::{
    McpServer,
    prompts::{NoPrompts, Prompts},
    resources::{NoResources, Resources},
    streamable_http::{Config, endpoint, endpoint_with_config},
    tool::{NoTools, Tools},
};
use serde_json::{Value, json};
use tokio_stream::StreamExt;

fn initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26", "capabilities": {},
            "clientInfo": { "name": "fallible-factory", "version": "1" }
        }
    })
}

async fn initialize<E: Endpoint>(client: &TestClient<E>) -> TestResponse {
    client
        .post("/")
        .header("Accept", "application/json, text/event-stream")
        .body_json(&initialize_request())
        .send()
        .await
}

async fn assert_initializes(app: impl IntoEndpoint) {
    let response = initialize(&TestClient::new(app)).await;
    response.assert_status_is_ok();
    response.assert_header_exist("Mcp-Session-Id");
    let body: Value = response.0.into_body().into_json().await.unwrap();
    assert_eq!(body["result"]["serverInfo"]["name"], "factory");
}

fn plain_factory(request: &Request) -> McpServer {
    assert_eq!(request.method(), Method::POST);
    McpServer::new().with_server_info("factory", "1")
}

fn fallible_factory(request: &Request) -> poem::Result<McpServer> {
    Ok(plain_factory(request))
}

// Preserve both the original four generic arguments and generic downstream
// wrappers whose bounds still describe an infallible factory.
fn legacy_wrapper<F, T, P, R>(factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<T, P, R> + Send + Sync + 'static,
    T: Tools + Send + Sync + 'static,
    P: Prompts + Send + Sync + 'static,
    R: Resources + Send + Sync + 'static,
{
    endpoint::<F, T, P, R>(factory)
}

fn legacy_config_wrapper<F, T, P, R>(factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<T, P, R> + Send + Sync + 'static,
    T: Tools + Send + Sync + 'static,
    P: Prompts + Send + Sync + 'static,
    R: Resources + Send + Sync + 'static,
{
    endpoint_with_config::<F, T, P, R>(factory, Config::default())
}

#[tokio::test]
async fn plain_and_fallible_factories_preserve_type_inference_and_generic_arity() {
    assert_initializes(endpoint(|request| {
        assert_eq!(request.method(), Method::POST);
        plain_factory(request)
    }))
    .await;
    assert_initializes(endpoint_with_config(
        |request| {
            assert_eq!(request.method(), Method::POST);
            plain_factory(request)
        },
        Config::default(),
    ))
    .await;
    assert_initializes(endpoint(|request| {
        assert_eq!(request.method(), Method::POST);
        Ok::<_, poem::Error>(plain_factory(request))
    }))
    .await;
    assert_initializes(endpoint_with_config(
        |request| {
            assert_eq!(request.method(), Method::POST);
            Ok::<_, StatusCode>(plain_factory(request))
        },
        Config::default(),
    ))
    .await;

    assert_initializes(endpoint::<_, NoTools, NoPrompts, NoResources>(|_| {
        McpServer::new().with_server_info("factory", "1")
    }))
    .await;
    assert_initializes(endpoint_with_config::<_, NoTools, NoPrompts, NoResources>(
        |_| McpServer::new().with_server_info("factory", "1"),
        Config::default(),
    ))
    .await;
    assert_initializes(endpoint::<_, NoTools, NoPrompts, NoResources>(
        fallible_factory,
    ))
    .await;
    assert_initializes(endpoint_with_config::<_, NoTools, NoPrompts, NoResources>(
        fallible_factory,
        Config::default(),
    ))
    .await;
    assert_initializes(endpoint(plain_factory)).await;
    assert_initializes(endpoint(plain_factory as fn(&Request) -> McpServer)).await;
    assert_initializes(endpoint(fallible_factory)).await;
    assert_initializes(endpoint_with_config(
        fallible_factory as fn(&Request) -> poem::Result<McpServer>,
        Config::default(),
    ))
    .await;
    assert_initializes(legacy_wrapper(plain_factory)).await;
    assert_initializes(legacy_config_wrapper(plain_factory)).await;
}

fn unauthorized() -> poem::Error {
    poem::Error::from_response(
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", "Bearer realm=\"mcp\"")
            .body("credentials required"),
    )
}

#[tokio::test]
async fn failed_initialization_can_retry_and_existing_sessions_do_not_rerun_factory() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = calls.clone();
    let client = TestClient::new(endpoint(move |request| {
        factory_calls.fetch_add(1, Ordering::SeqCst);
        let name = request
            .headers()
            .get("x-server-name")
            .ok_or_else(unauthorized)?;
        Ok::<_, poem::Error>(McpServer::new().with_server_info(name.to_str().unwrap(), "1"))
    }));

    let rejected = initialize(&client).await;
    rejected.assert_status(StatusCode::UNAUTHORIZED);
    rejected.assert_header("WWW-Authenticate", "Bearer realm=\"mcp\"");
    rejected.assert_header_is_not_exist("Mcp-Session-Id");
    rejected.assert_text("credentials required").await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let response = client
        .post("/")
        .header("x-server-name", "retry")
        .header("Accept", "application/json")
        .body_json(&initialize_request())
        .send()
        .await;
    response.assert_status_is_ok();
    let session_id = response.0.headers()["Mcp-Session-Id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.0.into_body().into_json().await.unwrap();
    assert_eq!(body["result"]["serverInfo"]["name"], "retry");

    // These requests lack the header required by the factory. Session creation
    // checks are deliberately not per-request authentication.
    let response = client
        .get("/")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await;
    response.assert_status_is_ok();
    let mut notifications = response.sse_stream();
    let Event::Message { data, .. } = notifications.next().await.unwrap() else {
        panic!("expected endpoint event");
    };
    assert_eq!(data, format!("?session_id={session_id}"));
    client
        .post("/")
        .header("Mcp-Session-Id", &session_id)
        .header("Accept", "application/json")
        .body_json(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        .send()
        .await
        .assert_json(json!([{"jsonrpc": "2.0", "id": 2, "result": {}}]))
        .await;
    drop(notifications);
    client
        .delete("/")
        .header("Mcp-Session-Id", &session_id)
        .send()
        .await
        .assert_status(StatusCode::ACCEPTED);
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    for method in [Method::GET, Method::POST] {
        client
            .request(method, "/")
            .header("Mcp-Session-Id", &session_id)
            .body_json(&initialize_request())
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn legacy_sse_can_retry_after_failure_and_keeps_response_routing() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = calls.clone();
    let client = TestClient::new(endpoint_with_config(
        move |request| {
            factory_calls.fetch_add(1, Ordering::SeqCst);
            request
                .headers()
                .get("x-allow-session")
                .ok_or_else(unauthorized)?;
            Ok::<_, poem::Error>(McpServer::new())
        },
        Config {
            session_timeout: None,
        },
    ));

    let rejected = client.get("/").send().await;
    rejected.assert_status(StatusCode::UNAUTHORIZED);
    rejected.assert_header("WWW-Authenticate", "Bearer realm=\"mcp\"");
    rejected.assert_header_is_not_exist("Mcp-Session-Id");
    assert_ne!(rejected.0.content_type(), Some("text/event-stream"));
    rejected.assert_text("credentials required").await;

    let response = client
        .get("/")
        .header("x-allow-session", "yes")
        .send()
        .await;
    response.assert_status_is_ok();
    let mut notifications = response.sse_stream();
    let Event::Message { data: endpoint, .. } = notifications.next().await.unwrap() else {
        panic!("expected endpoint event");
    };
    client
        .post(format!("/{endpoint}"))
        .body_json(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        .send()
        .await
        .assert_status(StatusCode::ACCEPTED);
    let Event::Message { data, .. } = notifications.next().await.unwrap() else {
        panic!("expected response event");
    };
    assert_eq!(
        serde_json::from_str::<Value>(&data).unwrap(),
        json!({"jsonrpc": "2.0", "id": 2, "result": {}})
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    drop(notifications);
    client
        .delete(format!("/{endpoint}"))
        .send()
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn errors_convert_into_poem_errors_for_both_transports() {
    let client = TestClient::new(endpoint(|_| Err::<McpServer, _>(StatusCode::FORBIDDEN)));
    for method in [Method::GET, Method::POST] {
        let response = client
            .request(method, "/")
            .header("Accept", "application/json")
            .body_json(&initialize_request())
            .send()
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        response.assert_header_is_not_exist("Mcp-Session-Id");
    }
}

#[derive(Debug)]
struct FactoryError;

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("factory failed")
    }
}

impl std::error::Error for FactoryError {}

impl poem::error::ResponseError for FactoryError {
    fn status(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}

#[tokio::test]
async fn factory_errors_reach_poem_error_middleware() {
    let caught = Arc::new(AtomicUsize::new(0));
    let middleware_caught = caught.clone();
    let client = TestClient::new(
        endpoint(|_| Err::<McpServer, _>(FactoryError))
            .into_endpoint()
            .catch_all_error(move |error| {
                assert!(error.downcast_ref::<FactoryError>().is_some());
                assert_eq!(error.status(), StatusCode::FORBIDDEN);
                middleware_caught.fetch_add(1, Ordering::SeqCst);
                async move {
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header("x-caught-factory-error", "yes")
                        .body("handled by middleware")
                }
            }),
    );
    for method in [Method::GET, Method::POST] {
        let response = client
            .request(method, "/")
            .header("Accept", "application/json")
            .body_json(&initialize_request())
            .send()
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
        response.assert_header("x-caught-factory-error", "yes");
        response.assert_header_is_not_exist("Mcp-Session-Id");
        response.assert_text("handled by middleware").await;
    }
    assert_eq!(caught.load(Ordering::SeqCst), 2);
}
