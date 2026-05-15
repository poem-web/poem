//! Streamable HTTP endpoint for handling MCP requests.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
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
    server::ServerMetadata,
    tool::Tools,
};

const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 5);
const SSE_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);
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
    /// Cached shared metadata, populated lazily from the first server
    /// produced by `server_factory`. All subsequent sessions reuse this
    /// instance, so the heavy configuration (resources, server info, ...) is
    /// not duplicated across sessions.
    shared_metadata: OnceLock<Arc<ServerMetadata>>,
}

impl<ToolsType, PromptsType, ResourcesType> State<ToolsType, PromptsType, ResourcesType>
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    /// Create a fresh per-session [`McpServer`] from the configured factory,
    /// substituting its metadata with the shared cached instance to avoid
    /// duplicating the static configuration across sessions.
    fn make_server(&self, request: &Request) -> McpServer<ToolsType, PromptsType, ResourcesType> {
        let mut server = (self.server_factory)(request);
        let shared = self
            .shared_metadata
            .get_or_init(|| server.metadata().clone())
            .clone();
        server.set_metadata(shared);
        server
    }
}

/// On-drop guard for an SSE attachment.
///
/// In legacy SSE transport mode (`owns_session = true`) the session lifetime
/// is bound to the SSE stream, so dropping removes the whole session. In
/// streamable HTTP resume mode (`owns_session = false`) the session was
/// created by a POST `initialize` and survives the SSE detaching: dropping
/// only clears the per-session SSE sender so the cleanup task / next GET can
/// reattach.
struct SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
    session_id: String,
    owns_session: bool,
}

impl<ToolsType, PromptsType, ResourcesType> SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    fn owning(
        state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
        session_id: String,
    ) -> Self {
        Self {
            state,
            session_id,
            owns_session: true,
        }
    }

    fn attached(
        state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
        session_id: String,
    ) -> Self {
        Self {
            state,
            session_id,
            owns_session: false,
        }
    }
}

impl<ToolsType, PromptsType, ResourcesType> Drop
    for SessionCleanup<ToolsType, PromptsType, ResourcesType>
{
    fn drop(&mut self) {
        let mut sessions = self.state.sessions.lock().unwrap();
        if self.owns_session {
            if sessions.remove(&self.session_id).is_some() {
                tracing::info!(
                    session_id = self.session_id,
                    "cleaned up closed standard session"
                );
            }
        } else if let Some(session) = sessions.get_mut(&self.session_id) {
            session.sender = None;
            tracing::info!(
                session_id = self.session_id,
                "detached SSE from streamable HTTP session"
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
    let existing_session_id = request
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(String::from);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let (session_id, owns_session) = if let Some(existing) = existing_session_id {
        // Streamable HTTP "resume" path: attach SSE to an already-initialised
        // session, do NOT create a new server (which would leak the existing
        // one and duplicate state).
        let mut sessions = data.0.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(&existing) else {
            tracing::warn!(
                session_id = existing,
                "GET for unknown session id (expired or invalid)"
            );
            return StatusCode::NOT_FOUND.into_response();
        };
        // Replace any previous sender; dropping it will tear down the previous
        // SSE stream (if any), which is the desired behaviour for resume.
        session.sender = Some(tx);
        session.last_active = Instant::now();
        tracing::info!(session_id = existing, "attached SSE to existing session");
        (existing, false)
    } else {
        // Legacy SSE transport: create a brand new session keyed off the SSE
        // connection itself.
        let session_id = session_id();
        let server = data.0.make_server(request);
        let mut sessions = data.0.sessions.lock().unwrap();
        sessions.insert(
            session_id.clone(),
            Session {
                server: Arc::new(tokio::sync::Mutex::new(server)),
                sender: Some(tx),
                last_active: Instant::now(),
            },
        );
        tracing::info!(session_id, "created new standard session (SSE)");
        (session_id, true)
    };

    let state = data.0.clone();
    let cleanup_session_id = session_id.clone();
    SSE::new(async_stream::stream! {
        let _cleanup = if owns_session {
            SessionCleanup::owning(state, cleanup_session_id)
        } else {
            SessionCleanup::attached(state, cleanup_session_id)
        };
        let endpoint_uri = format!("?session_id={}", session_id);
        yield Event::message(endpoint_uri).event_type("endpoint");

        while let Some(msg) = rx.recv().await {
             yield Event::message(msg).event_type("message");
        }
    })
    .keep_alive(SSE_KEEP_ALIVE_INTERVAL)
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
            let mut server = data.0.make_server(request);
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
            .keep_alive(SSE_KEEP_ALIVE_INTERVAL)
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
/// Standard SSE sessions are still cleaned up when the stream closes. The SSE
/// stream emits periodic keep-alive comments so that silently disconnected
/// clients (e.g. the client process was killed without sending `DELETE`) are
/// detected through a failing socket write and their session is reclaimed
/// promptly. HTTP-only Streamable HTTP sessions do not have a persistent
/// connection that can signal client process exit, so clients must explicitly
/// `DELETE` them if idle expiration is disabled.
///
/// # Shared configuration
///
/// The static configuration of the [`McpServer`] returned by `server_factory`
/// (server info, registered resources, disabled tools, ...) is captured from
/// the first invocation and shared across every session via an [`Arc`]. This
/// keeps the per-session memory footprint small even when serving large
/// embedded resources. The per-session mutable state of your `Tools` /
/// `Prompts` / `Resources` implementations is still produced fresh by the
/// factory on every new session.
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
        shared_metadata: OnceLock::new(),
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

    #[tokio::test]
    async fn get_with_session_id_attaches_to_existing_session() {
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
                    "clientInfo": { "name": "test-client", "version": "1.0.0" }
                }
            }))
            .send()
            .await;
        resp.assert_status_is_ok();
        let session_id = resp
            .0
            .headers()
            .get("Mcp-Session-Id")
            .expect("session id header")
            .to_str()
            .expect("valid session id")
            .to_string();

        // Resuming with the session id MUST NOT create a new session.
        let resume = cli
            .get("/")
            .header("Mcp-Session-Id", &session_id)
            .send()
            .await;
        resume.assert_status_is_ok();
        let mut stream = resume.sse_stream();
        let event = stream.next().await.expect("endpoint event");
        let echoed_session = match event {
            poem::web::sse::Event::Message { data, .. } => data
                .strip_prefix("?session_id=")
                .expect("session id payload")
                .to_string(),
            poem::web::sse::Event::Retry { .. } => panic!("unexpected retry event"),
        };
        assert_eq!(echoed_session, session_id);

        // Dropping the stream should release the SSE attachment but keep the
        // underlying session intact, since the streamable HTTP session is owned
        // by the POST init, not by the SSE GET.
        drop(stream);
        tokio::time::sleep(Duration::from_millis(50)).await;

        cli.delete("/")
            .header("Mcp-Session-Id", &session_id)
            .send()
            .await
            .assert_status(StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn get_with_unknown_session_id_returns_not_found() {
        let app = endpoint_with_config(
            |_| McpServer::new(),
            Config {
                session_timeout: None,
            },
        );
        let cli = TestClient::new(app);
        cli.get("/")
            .header("Mcp-Session-Id", "deadbeef")
            .send()
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }
}
