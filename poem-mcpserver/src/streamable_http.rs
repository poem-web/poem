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
    protocol::rpc::{BatchRequest as McpBatchRequest, Request as McpRequest},
    tool::Tools,
};

const SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);

type ServerFactoryFn<ToolsType> = Box<dyn Fn(&Request) -> McpServer<ToolsType> + Send + Sync>;

struct Session<ToolsType> {
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType>>>,
    last_active: Instant,
}

struct State<ToolsType> {
    server_factory: ServerFactoryFn<ToolsType>,
    sessions: Mutex<HashMap<String, Session<ToolsType>>>,
}

async fn handle_request<ToolsType>(
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType>>>,
    session_id: &str,
    accept: &Mime,
    requests: impl Iterator<Item = McpRequest> + Send + 'static,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
{
    tracing::info!(
        session_id = session_id,
        accept = accept.essence_str(),
        "handling requests"
    );

    match accept.essence_str() {
        "application/json" => {
            let mut resps = vec![];
            for request in requests {
                tracing::info!(session_id = session_id, request = ?request, "received request");
                let resp = server.lock().await.handle_request(request).await;
                tracing::info!(session_id = session_id, response = ?resp, "sending response");
                resps.extend(resp);
            }
            Json(resps)
                .with_content_type("application/json")
                .into_response()
        }
        "text/event-stream" => {
            let session_id = session_id.to_string();
            SSE::new(async_stream::stream! {
                for request in requests {
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
async fn post_handler<ToolsType>(
    data: Data<&Arc<State<ToolsType>>>,
    request: &Request,
    batch_request: Json<McpBatchRequest>,
    accept: Accept,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
{
    let Some(accept) = accept.0.first() else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    if batch_request.len() == 1
        && batch_request.requests()[0].is_initialize()
        && !request.headers().contains_key("Mcp-Session-Id")
    {
        let session_id = session_id();
        let mut server = (data.0.server_factory)(request);
        let initialize_request = batch_request.0.into_iter().next().unwrap();
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

    handle_request(server, session_id, &accept, batch_request.0.into_iter())
        .await
        .into_response()
}

#[handler]
async fn delete_handler<ToolsType>(
    data: Data<&Arc<State<ToolsType>>>,
    headers: &HeaderMap,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
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
pub fn endpoint<F, ToolsType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
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

    post(post_handler::<ToolsType>::default())
        .delete(delete_handler::<ToolsType>::default())
        .data(state)
}

fn session_id() -> String {
    format!("{:016x}", rand::random::<u128>())
}
