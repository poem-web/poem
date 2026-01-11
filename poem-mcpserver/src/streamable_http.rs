//! Streamable HTTP endpoint for handling MCP requests.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use poem::{
    EndpointExt, IntoEndpoint, IntoResponse, Request, handler,
    http::StatusCode,
    post,
    web::{
        Accept, Data, Json, Query,
        sse::{Event, SSE},
    },
};
use serde_json::Value;
use tokio::time::Instant;

use crate::{
    McpServer,
    prompts::Prompts,
    protocol::rpc::{BatchRequest as McpBatchRequest, Request as McpRequest},
    tool::Tools,
};

const SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);

type ServerFactoryFn<ToolsType, PromptsType> =
    Box<dyn Fn(&Request) -> McpServer<ToolsType, PromptsType> + Send + Sync>;

struct Session<ToolsType, PromptsType> {
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType>>>,
    sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    last_active: Instant,
}

struct State<ToolsType, PromptsType> {
    server_factory: ServerFactoryFn<ToolsType, PromptsType>,
    sessions: Mutex<HashMap<String, Session<ToolsType, PromptsType>>>,
}

async fn process_request<ToolsType, PromptsType>(
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType>>>,
    request: McpRequest,
) -> Option<crate::protocol::rpc::Response<Value>>
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    server.lock().await.handle_request(request).await
}

#[handler]
async fn get_handler<ToolsType, PromptsType>(
    data: Data<&Arc<State<ToolsType, PromptsType>>>,
    request: &Request,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let session_id = session_id();
    let server = (data.0.server_factory)(request);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    {
        let mut sessions = data.0.sessions.lock().unwrap();
        sessions.insert(
            session_id.clone(),
            Session {
                server: Arc::new(tokio::sync::Mutex::new(server)),
                sender: Some(tx),
                last_active: Instant::now(),
            },
        );
    }

    tracing::info!(
        session_id = session_id,
        "created new standard session (SSE)"
    );

    SSE::new(async_stream::stream! {
        let endpoint_uri = format!("?session_id={}", session_id);
        yield Event::message(endpoint_uri).event_type("endpoint");

        while let Some(msg) = rx.recv().await {
             yield Event::message(msg).event_type("message");
        }
    })
    .into_response()
}

#[handler]
async fn post_handler<ToolsType, PromptsType>(
    data: Data<&Arc<State<ToolsType, PromptsType>>>,
    request: &Request,
    batch_request: Json<McpBatchRequest>,
    accept: Accept,
    query: Query<HashMap<String, String>>,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let session_id_param = request
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(String::from)
        .or_else(|| query.get("session_id").cloned());

    if session_id_param.is_none() {
        let Some(_accept) = accept.0.first() else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        if batch_request.len() == 1 && batch_request.requests()[0].is_initialize() {
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
                    sender: None,
                    last_active: Instant::now(),
                },
            );

            tracing::info!(session_id = session_id, "created new legacy session");
            return Json(resp)
                .with_header("Mcp-Session-Id", session_id)
                .into_response();
        }

        return StatusCode::BAD_REQUEST.into_response();
    }

    let session_id = session_id_param.unwrap();

    let (server, sender) = {
        let mut sessions = data.0.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(&session_id) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        session.last_active = Instant::now();
        (session.server.clone(), session.sender.clone())
    };

    if let Some(tx) = sender {
        for request in batch_request.0 {
            tracing::info!(session_id = session_id, request = ?request, "received request (std)");
            let resp = process_request(server.clone(), request).await;
            if let Some(resp) = resp {
                tracing::info!(session_id = session_id, response = ?resp, "pushing to SSE");
                if tx.send(serde_json::to_string(&resp).unwrap()).is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
        return StatusCode::ACCEPTED.into_response();
    }

    let Some(accept) = accept.0.first() else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let requests = batch_request.0.into_iter();

    match accept.essence_str() {
        "text/event-stream" => {
            let session_id = session_id.clone();
            SSE::new(async_stream::stream! {
                for request in requests {
                    tracing::info!(session_id = session_id, request = ?request, "received request");
                    let resp = process_request(server.clone(), request).await;
                    if let Some(resp) = resp {
                        tracing::info!(session_id = session_id, response = ?resp, "sending response");
                        yield Event::message(serde_json::to_string(&resp).unwrap()).event_type("message");
                    }
                }
            })
            .into_response()
        }
        _ => {
            let mut resps = vec![];
            for request in requests {
                tracing::info!(session_id = session_id, request = ?request, "received request");
                let resp = process_request(server.clone(), request).await;
                if let Some(resp) = resp {
                    tracing::info!(session_id = session_id, response = ?resp, "sending response");
                    resps.push(resp);
                }
            }
            Json(resps)
                .with_content_type("application/json")
                .into_response()
        }
    }
}

#[handler]
async fn delete_handler<ToolsType, PromptsType>(
    data: Data<&Arc<State<ToolsType, PromptsType>>>,
    req: &Request,
    query: Query<HashMap<String, String>>,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let session_id = req
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(String::from)
        .or_else(|| query.get("session_id").cloned());

    let Some(session_id) = session_id else {
        return StatusCode::BAD_REQUEST;
    };

    if data
        .0
        .sessions
        .lock()
        .unwrap()
        .remove(&session_id)
        .is_none()
    {
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
        .get(get_handler::<ToolsType, PromptsType>::default())
        .delete(delete_handler::<ToolsType, PromptsType>::default())
        .data(state)
}

fn session_id() -> String {
    format!("{:016x}", rand::random::<u128>())
}
