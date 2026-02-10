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
    protocol::rpc::{BatchRequest as McpBatchRequest, Request as McpRequest, Requests},
    tool::Tools,
};

const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);

/// Configuration options for streamable HTTP sessions.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Session idle timeout. Use `None` to disable expiration.
    pub session_timeout: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session_timeout: Some(DEFAULT_SESSION_TIMEOUT),
        }
    }
}

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
            tracing::warn!(
                session_id = session_id,
                "session not found (expired or invalid)"
            );
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

    let all_notifications = batch_request.requests().iter().all(|request| {
        matches!(
            request.body,
            Requests::Initialized | Requests::Cancelled { .. }
        )
    });

    let requests = batch_request.0.into_iter();

    let accept = accept
        .0
        .first()
        .map(|value| value.essence_str())
        .unwrap_or("application/json");

    match accept {
        "text/event-stream" => {
            if all_notifications {
                return StatusCode::ACCEPTED.into_response();
            }
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
            if resps.is_empty() {
                return StatusCode::ACCEPTED.into_response();
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
///
/// Uses the default configuration (5-minute idle timeout).
pub fn endpoint<F, ToolsType, PromptsType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType, PromptsType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    endpoint_with_config(server_factory, Config::default())
}

/// A streamable http endpoint with configurable session behavior.
///
/// Set `Config::session_timeout` to `None` to disable session expiration.
///
/// # Example
/// ```rust,no_run
/// use poem::Route;
/// use poem_mcpserver::{McpServer, streamable_http};
///
/// let app = Route::new().at(
///     "/",
///     streamable_http::endpoint_with_config(
///         |_| McpServer::new(),
///         streamable_http::Config {
///             session_timeout: None,
///         },
///     ),
/// );
/// ```
pub fn endpoint_with_config<F, ToolsType, PromptsType>(
    server_factory: F,
    config: Config,
) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType, PromptsType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
{
    let state = Arc::new(State {
        server_factory: Box::new(server_factory),
        sessions: Default::default(),
    });

    let session_timeout = config.session_timeout;
    tokio::spawn({
        let state = state.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                let now = interval.tick().await;
                let mut sessions = state.sessions.lock().unwrap();
                sessions.retain(|session_id, session| {
                    let Some(timeout) = session_timeout else {
                        return true;
                    };
                    let expired = (now - session.last_active) >= timeout;
                    if expired {
                        tracing::info!(
                            session_id = session_id,
                            timeout_seconds = timeout.as_secs(),
                            last_active = ?session.last_active,
                            "expired session"
                        );
                    }
                    !expired
                });
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
