//! Streamable HTTP endpoint for handling MCP requests.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use mime::Mime;
use poem::{
    EndpointExt, IntoEndpoint, IntoResponse, Request, handler,
    http::{HeaderMap, StatusCode},
    post,
    web::{
        Accept, Data, Json,
        sse::{Event, SSE},
    },
};
use tokio::time::Instant;

use crate::{
    McpServer,
    prompts::Prompts,
    protocol::rpc::BatchRequest as McpBatchRequest,
    tool::Tools,
};

const SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);

type ServerFactoryFn<ToolsType, PromptsType> =
    Box<dyn Fn(&Request) -> McpServer<ToolsType, PromptsType> + Send + Sync>;

struct Session<ToolsType, PromptsType> {
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType>>>,
    last_active: Instant,
}

struct State<ToolsType, PromptsType> {
    server_factory: ServerFactoryFn<ToolsType, PromptsType>,
    sessions: Mutex<HashMap<String, Session<ToolsType, PromptsType>>>,
}

async fn handle_request<ToolsType, PromptsType>(
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType>>>,
    session_id: &str,
    accept: &Mime,
    batch_request: McpBatchRequest,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    tracing::info!(
        session_id = session_id,
        accept = accept.essence_str(),
        "handling requests"
    );

    match accept.essence_str() {
        "application/json" => {
            let is_batch = matches!(batch_request, McpBatchRequest::Batch(_));
            let mut resps = vec![];
            for request in batch_request.into_iter() {
                tracing::info!(session_id = session_id, request = ?request, "received request");
                let resp = server.lock().await.handle_request(request).await;
                tracing::info!(session_id = session_id, response = ?resp, "sending response");
                resps.extend(resp);
            }
            if is_batch || resps.len() != 1 {
                Json(resps)
                    .with_content_type("application/json")
                    .into_response()
            } else {
                Json(resps.pop().expect("BUG: missing single response"))
                    .with_content_type("application/json")
                    .into_response()
            }
        }
        "text/event-stream" => {
            let session_id = session_id.to_string();
            SSE::new(async_stream::stream! {
                for request in batch_request.into_iter() {
                    tracing::info!(session_id = session_id, request = ?request, "received request");
                    let resp = server.lock().await.handle_request(request).await;
                    tracing::info!(session_id = session_id, response = ?resp, "sending response");
                    yield Event::message(serde_json::to_string(&resp).unwrap()).event_type("message");
                }
            })
            .into_response()
        }
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

#[handler]
async fn post_handler<ToolsType, PromptsType>(
    data: Data<&Arc<State<ToolsType, PromptsType>>>,
    request: &Request,
    batch_request: Json<McpBatchRequest>,
    accept: Accept,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let Some(accept) = accept.0.first() else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let batch_request = batch_request.0;

    if batch_request.len() == 1
        && batch_request.requests()[0].is_initialize()
        && !request.headers().contains_key("Mcp-Session-Id")
    {
        let session_id = session_id();
        let mut server = (data.0.server_factory)(request);
        let initialize_request = batch_request.into_iter().next().unwrap();
        let resp = server
            .handle_request(initialize_request)
            .await
            .expect("BUG: initialize response");
        let mut sessions = data.0.sessions.lock().unwrap();
        sessions.insert(
            session_id.clone(),
            Session {
                server: Arc::new(tokio::sync::Mutex::new(server)),
                last_active: Instant::now(),
            },
        );

        tracing::info!(session_id = session_id, "created new session");
        return Json(resp)
            .with_header("Mcp-Session-Id", session_id)
            .into_response();
    }

    let Some(session_id) = request
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let server = {
        let mut sessions = data.0.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(session_id) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        session.last_active = Instant::now();
        session.server.clone()
    };

    handle_request(server, session_id, accept, batch_request)
        .await
        .into_response()
}

#[handler]
async fn delete_handler<ToolsType, PromptsType>(
    data: Data<&Arc<State<ToolsType, PromptsType>>>,
    headers: &HeaderMap,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let Some(session_id) = headers
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
    else {
        return StatusCode::BAD_REQUEST;
    };

    if data.sessions.lock().unwrap().remove(session_id).is_none() {
        return StatusCode::NOT_FOUND;
    }

    tracing::info!(session_id = session_id, "deleted session");
    StatusCode::ACCEPTED
}

/// A streamable http endpoint that can be used to handle MCP requests.
pub fn endpoint<F, ToolsType, PromptsType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType, PromptsType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let state = Arc::new(State {
        server_factory: Box::new(server_factory),
        sessions: Default::default(),
    });

    tokio::spawn({
        let state = state.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                let now = interval.tick().await;
                let mut sessions = state.sessions.lock().unwrap();
                sessions.retain(|_, session| (now - session.last_active) < SESSION_TIMEOUT);
            }
        }
    });

    post(post_handler::<ToolsType, PromptsType>::default())
        .delete(delete_handler::<ToolsType, PromptsType>::default())
        .data(state)
}

fn session_id() -> String {
    format!("{:016x}", rand::random::<u128>())
}
