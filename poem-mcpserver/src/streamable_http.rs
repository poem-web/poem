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
    resources::Resources,
    tool::Tools,
};

const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);
const REQUEST_LOG_TARGET: &str = "poem_mcpserver::payload::request";
const RESPONSE_LOG_TARGET: &str = "poem_mcpserver::payload::response";

/// Configuration options for streamable HTTP sessions.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Session idle timeout. Use `None` to disable idle expiration.
    ///
    /// HTTP-only Streamable HTTP sessions have no persistent connection that
    /// can signal client process exit. If idle expiration is disabled, clients
    /// should terminate those sessions with `DELETE` and `Mcp-Session-Id`.
    pub session_timeout: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session_timeout: Some(DEFAULT_SESSION_TIMEOUT),
        }
    }
}

type ServerFactoryFn<ToolsType, PromptsType, ResourcesType> =
    Box<dyn Fn(&Request) -> McpServer<ToolsType, PromptsType, ResourcesType> + Send + Sync>;

struct Session<ToolsType, PromptsType, ResourcesType> {
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType, ResourcesType>>>,
    sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    last_active: Instant,
}

struct State<ToolsType, PromptsType, ResourcesType> {
    server_factory: ServerFactoryFn<ToolsType, PromptsType, ResourcesType>,
    sessions: Mutex<HashMap<String, Session<ToolsType, PromptsType, ResourcesType>>>,
}

struct SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
    session_id: String,
}

impl<ToolsType, PromptsType, ResourcesType> SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    fn new(state: Arc<State<ToolsType, PromptsType, ResourcesType>>, session_id: String) -> Self {
        Self { state, session_id }
    }
}

impl<ToolsType, PromptsType, ResourcesType> Drop
    for SessionCleanup<ToolsType, PromptsType, ResourcesType>
{
    fn drop(&mut self) {
        if self
            .state
            .sessions
            .lock()
            .unwrap()
            .remove(&self.session_id)
            .is_some()
        {
            tracing::info!(
                session_id = self.session_id,
                "cleaned up closed standard session"
            );
        }
    }
}

async fn process_request<ToolsType, PromptsType, ResourcesType>(
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType, ResourcesType>>>,
    request: McpRequest,
) -> Option<crate::protocol::rpc::Response<Value>>
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    server.lock().await.handle_request(request).await
}

#[handler]
async fn get_handler<ToolsType, PromptsType, ResourcesType>(
    data: Data<&Arc<State<ToolsType, PromptsType, ResourcesType>>>,
    request: &Request,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
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

    tracing::info!(session_id, "created new standard session (SSE)");

    let state = data.0.clone();
    let cleanup_session_id = session_id.clone();
    SSE::new(async_stream::stream! {
        let _cleanup = SessionCleanup::new(state, cleanup_session_id);
        let endpoint_uri = format!("?session_id={}", session_id);
        yield Event::message(endpoint_uri).event_type("endpoint");

        while let Some(msg) = rx.recv().await {
             yield Event::message(msg).event_type("message");
        }
    })
    .into_response()
}

#[handler]
async fn post_handler<ToolsType, PromptsType, ResourcesType>(
    data: Data<&Arc<State<ToolsType, PromptsType, ResourcesType>>>,
    request: &Request,
    batch_request: Json<McpBatchRequest>,
    accept: Accept,
    query: Query<HashMap<String, String>>,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
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

            tracing::info!(session_id, "created new streamable HTTP session");
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
            tracing::warn!(session_id, "session not found (expired or invalid)");
            return StatusCode::NOT_FOUND.into_response();
        };
        session.last_active = Instant::now();
        (session.server.clone(), session.sender.clone())
    };

    if let Some(tx) = sender {
        for request in batch_request.0 {
            tracing::info!(
                target: REQUEST_LOG_TARGET,
                session_id,
                ?request,
                "received request (std)"
            );
            let resp = process_request(server.clone(), request).await;
            if let Some(resp) = resp {
                tracing::info!(
                    target: RESPONSE_LOG_TARGET,
                    session_id,
                    response = ?resp,
                    "pushing to SSE"
                );
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
                    tracing::info!(
                        target: REQUEST_LOG_TARGET,
                        session_id = session_id,
                        request = ?request,
                        "received request"
                    );
                    let resp = process_request(server.clone(), request).await;
                    if let Some(resp) = resp {
                        tracing::info!(
                            target: RESPONSE_LOG_TARGET,
                            session_id = session_id,
                            response = ?resp,
                            "sending response"
                        );
                        yield Event::message(serde_json::to_string(&resp).unwrap()).event_type("message");
                    }
                }
            })
            .into_response()
        }
        _ => {
            let mut resps = vec![];
            for request in requests {
                tracing::info!(
                    target: REQUEST_LOG_TARGET,
                    session_id = session_id,
                    request = ?request,
                    "received request"
                );
                let resp = process_request(server.clone(), request).await;
                if let Some(resp) = resp {
                    tracing::info!(
                        target: RESPONSE_LOG_TARGET,
                        session_id = session_id,
                        response = ?resp,
                        "sending response"
                    );
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
async fn delete_handler<ToolsType, PromptsType, ResourcesType>(
    data: Data<&Arc<State<ToolsType, PromptsType, ResourcesType>>>,
    req: &Request,
    query: Query<HashMap<String, String>>,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
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
pub fn endpoint<F, ToolsType, PromptsType, ResourcesType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType, PromptsType, ResourcesType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    endpoint_with_config(server_factory, Config::default())
}

/// A streamable http endpoint with configurable session behavior.
///
/// Set `Config::session_timeout` to `None` to disable idle expiration.
///
/// Standard SSE sessions are still cleaned up when the stream closes. HTTP-only
/// Streamable HTTP sessions do not have a persistent connection that can signal
/// client process exit, so clients must explicitly `DELETE` them if idle
/// expiration is disabled.
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
pub fn endpoint_with_config<F, ToolsType, PromptsType, ResourcesType>(
    server_factory: F,
    config: Config,
) -> impl IntoEndpoint
where
    F: Fn(&Request) -> McpServer<ToolsType, PromptsType, ResourcesType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    let state = Arc::new(State {
        server_factory: Box::new(server_factory),
        sessions: Default::default(),
    });

    let session_timeout = config.session_timeout;
    tokio::spawn({
        let state = Arc::downgrade(&state);
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                let now = interval.tick().await;
                let Some(state) = state.upgrade() else {
                    break;
                };
                let mut sessions = state.sessions.lock().unwrap();
                sessions.retain(|session_id, session| {
                    if session
                        .sender
                        .as_ref()
                        .is_some_and(|sender| sender.is_closed())
                    {
                        tracing::info!(
                            session_id = session_id,
                            "cleaned up closed standard session"
                        );
                        return false;
                    }

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

    post(post_handler::<ToolsType, PromptsType, ResourcesType>::default())
        .get(get_handler::<ToolsType, PromptsType, ResourcesType>::default())
        .delete(delete_handler::<ToolsType, PromptsType, ResourcesType>::default())
        .data(state)
}

fn session_id() -> String {
    format!("{:016x}", rand::random::<u128>())
}

#[cfg(all(test, feature = "streamable-http"))]
mod tests {
    use std::time::Duration;

    use poem::{http::StatusCode, test::TestClient};
    use serde_json::json;
    use tokio_stream::StreamExt;

    use super::{Config, endpoint_with_config};
    use crate::McpServer;

    #[tokio::test]
    async fn closes_standard_session_when_sse_stream_is_dropped() {
        let app = endpoint_with_config(
            |_| McpServer::new(),
            Config {
                session_timeout: None,
            },
        );
        let cli = TestClient::new(app);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();

        let mut stream = resp.sse_stream();
        let session_event = stream.next().await.expect("endpoint event");
        let session_id = match session_event {
            poem::web::sse::Event::Message { data, .. } => data
                .strip_prefix("?session_id=")
                .expect("session id payload")
                .to_string(),
            poem::web::sse::Event::Retry { .. } => panic!("unexpected retry event"),
        };

        drop(stream);
        tokio::time::sleep(Duration::from_millis(50)).await;

        cli.delete("/")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn http_session_without_timeout_requires_explicit_delete() {
        let app = endpoint_with_config(
            |_| McpServer::new(),
            Config {
                session_timeout: None,
            },
        );
        let cli = TestClient::new(app);

        let resp = cli
            .post("/")
            .header("Accept", "application/json")
            .content_type("application/json")
            .body_json(&json!({
                "jsonrpc": "2.0",
                "id": "init",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header_exist("Mcp-Session-Id");
        let session_id = resp
            .0
            .headers()
            .get("Mcp-Session-Id")
            .expect("session id header")
            .to_str()
            .expect("valid session id")
            .to_string();

        tokio::time::sleep(Duration::from_millis(50)).await;

        cli.post("/")
            .header("Mcp-Session-Id", &session_id)
            .header("Accept", "application/json")
            .content_type("application/json")
            .body_json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .send()
            .await
            .assert_status(StatusCode::ACCEPTED);

        cli.delete("/")
            .header("Mcp-Session-Id", &session_id)
            .send()
            .await
            .assert_status(StatusCode::ACCEPTED);

        cli.post("/")
            .header("Mcp-Session-Id", session_id)
            .header("Accept", "application/json")
            .content_type("application/json")
            .body_json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }
}
