//! Streamable HTTP endpoint for handling MCP requests.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use poem::{
    EndpointExt, IntoEndpoint, IntoResponse, Request, Response, handler,
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

/// A server or a fallible result returned by a streamable HTTP server factory.
///
/// Implemented for [`McpServer`] and `Result<McpServer, E>` where `E` can be
/// converted into [`poem::Error`].
pub trait IntoMcpServer<ToolsType, PromptsType, ResourcesType> {
    /// Converts the factory output into a server or a Poem error.
    fn into_mcp_server(self) -> poem::Result<McpServer<ToolsType, PromptsType, ResourcesType>>;
}

impl<ToolsType, PromptsType, ResourcesType> IntoMcpServer<ToolsType, PromptsType, ResourcesType>
    for McpServer<ToolsType, PromptsType, ResourcesType>
{
    fn into_mcp_server(self) -> poem::Result<Self> {
        Ok(self)
    }
}

impl<ToolsType, PromptsType, ResourcesType, E> IntoMcpServer<ToolsType, PromptsType, ResourcesType>
    for Result<McpServer<ToolsType, PromptsType, ResourcesType>, E>
where
    E: Into<poem::Error>,
{
    fn into_mcp_server(self) -> poem::Result<McpServer<ToolsType, PromptsType, ResourcesType>> {
        self.map_err(Into::into)
    }
}

/// A synchronous factory for a new streamable HTTP session's server.
///
/// Automatically implemented for functions and closures accepting a [`Request`]
/// reference and returning either an [`McpServer`] or `Result<McpServer, E>`
/// where `E` can be converted into [`poem::Error`].
pub trait McpServerFactory<ToolsType, PromptsType, ResourcesType>:
    Fn(&Request) -> Self::Return
{
    /// The server or fallible result produced by the factory.
    type Return: IntoMcpServer<ToolsType, PromptsType, ResourcesType>;
}

impl<F, R, ToolsType, PromptsType, ResourcesType>
    McpServerFactory<ToolsType, PromptsType, ResourcesType> for F
where
    F: Fn(&Request) -> R,
    R: IntoMcpServer<ToolsType, PromptsType, ResourcesType>,
{
    type Return = R;
}

type ServerFactoryFn<ToolsType, PromptsType, ResourcesType> = Box<
    dyn Fn(&Request) -> poem::Result<McpServer<ToolsType, PromptsType, ResourcesType>>
        + Send
        + Sync,
>;

struct Session<ToolsType, PromptsType, ResourcesType> {
    server: Arc<tokio::sync::Mutex<McpServer<ToolsType, PromptsType, ResourcesType>>>,
    /// Only legacy SSE sessions deliver POST responses on the GET stream.
    legacy_sse: bool,
    sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    /// Monotonic counter incremented every time a new SSE `sender` is
    /// installed on this session. The attached [`SessionCleanup`] guard
    /// remembers the generation it observed at attach time and only clears
    /// `sender` on drop when it still owns the current generation, so a
    /// stale guard cannot wipe out a sender that was replaced by a newer GET.
    sender_gen: u64,
    last_active: Instant,
}

struct State<ToolsType, PromptsType, ResourcesType> {
    server_factory: ServerFactoryFn<ToolsType, PromptsType, ResourcesType>,
    sessions: Mutex<HashMap<String, Session<ToolsType, PromptsType, ResourcesType>>>,
    /// Cached shared metadata, populated lazily from the first server
    /// produced by `server_factory`. Subsequent sessions with identical
    /// metadata reuse this instance; request-specific metadata stays isolated.
    shared_metadata: OnceLock<Arc<ServerMetadata>>,
}

impl<ToolsType, PromptsType, ResourcesType> State<ToolsType, PromptsType, ResourcesType>
where
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    /// Create a fresh per-session [`McpServer`] from the configured factory,
    /// reusing the cached metadata only when its contents are identical.
    fn make_server(
        &self,
        request: &Request,
    ) -> poem::Result<McpServer<ToolsType, PromptsType, ResourcesType>> {
        let mut server = (self.server_factory)(request)?;
        let shared = self
            .shared_metadata
            .get_or_init(|| server.metadata().clone());
        if server.metadata().as_ref() == shared.as_ref() {
            server.set_metadata(shared.clone());
        }
        Ok(server)
    }
}

/// On-drop guard for an SSE attachment.
///
/// In legacy SSE transport mode ([`SessionCleanupKind::Owning`]) the session
/// lifetime is bound to the SSE stream, so dropping removes the whole
/// session. In streamable HTTP resume mode ([`SessionCleanupKind::Attached`])
/// the session was created by a POST `initialize` and survives the SSE
/// detaching: dropping only clears the per-session SSE sender, and only when
/// the recorded generation still matches — this prevents a stale guard
/// (whose stream already lost the channel to a newer GET) from wiping out
/// the live sender installed by the newer attachment.
struct SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
    session_id: String,
    kind: SessionCleanupKind,
}

enum SessionCleanupKind {
    Owning,
    Attached { sender_gen: u64 },
}

impl<ToolsType, PromptsType, ResourcesType> SessionCleanup<ToolsType, PromptsType, ResourcesType> {
    fn owning(
        state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
        session_id: String,
    ) -> Self {
        Self {
            state,
            session_id,
            kind: SessionCleanupKind::Owning,
        }
    }

    fn attached(
        state: Arc<State<ToolsType, PromptsType, ResourcesType>>,
        session_id: String,
        sender_gen: u64,
    ) -> Self {
        Self {
            state,
            session_id,
            kind: SessionCleanupKind::Attached { sender_gen },
        }
    }
}

impl<ToolsType, PromptsType, ResourcesType> Drop
    for SessionCleanup<ToolsType, PromptsType, ResourcesType>
{
    fn drop(&mut self) {
        let mut sessions = self.state.sessions.lock().unwrap();
        match self.kind {
            SessionCleanupKind::Owning => {
                if sessions.remove(&self.session_id).is_some() {
                    tracing::info!(
                        session_id = self.session_id,
                        "cleaned up closed standard session"
                    );
                }
            }
            SessionCleanupKind::Attached { sender_gen } => {
                if let Some(session) = sessions.get_mut(&self.session_id) {
                    // Only clear the sender if it still belongs to *this*
                    // attachment. A newer GET may have replaced it, in which
                    // case the newer guard is responsible for it.
                    if session.sender_gen == sender_gen {
                        session.sender = None;
                        tracing::info!(
                            session_id = self.session_id,
                            "detached SSE from streamable HTTP session"
                        );
                    } else {
                        tracing::debug!(
                            session_id = self.session_id,
                            stale_gen = sender_gen,
                            current_gen = session.sender_gen,
                            "stale SSE attachment guard skipped"
                        );
                    }
                }
            }
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
) -> poem::Result<Response>
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

    let (session_id, attachment) = if let Some(existing) = existing_session_id {
        // Streamable HTTP "resume" path: attach SSE to an already-initialised
        // session, do NOT create a new server (which would leak the existing
        // one and duplicate state).
        let mut sessions = data.0.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(&existing) else {
            tracing::warn!(
                session_id = existing,
                "GET for unknown session id (expired or invalid)"
            );
            return Ok(StatusCode::NOT_FOUND.into_response());
        };
        // Replace any previous sender; dropping it will tear down the previous
        // SSE stream (if any), which is the desired behaviour for resume. Bump
        // the generation so the previous attachment's drop guard becomes a
        // no-op and cannot wipe out our freshly installed sender.
        session.sender = Some(tx);
        session.sender_gen = session.sender_gen.wrapping_add(1);
        session.last_active = Instant::now();
        let sender_gen = session.sender_gen;
        tracing::info!(session_id = existing, "attached SSE to existing session");
        (existing, Some(sender_gen))
    } else {
        // Legacy SSE transport: create a brand new session keyed off the SSE
        // connection itself.
        let server = data.0.make_server(request)?;
        let session_id = session_id();
        let mut sessions = data.0.sessions.lock().unwrap();
        sessions.insert(
            session_id.clone(),
            Session {
                server: Arc::new(tokio::sync::Mutex::new(server)),
                legacy_sse: true,
                sender: Some(tx),
                sender_gen: 0,
                last_active: Instant::now(),
            },
        );
        tracing::info!(session_id, "created new standard session (SSE)");
        (session_id, None)
    };

    let state = data.0.clone();
    let cleanup_session_id = session_id.clone();
    Ok(SSE::new(async_stream::stream! {
        let _cleanup = match attachment {
            None => SessionCleanup::owning(state, cleanup_session_id),
            Some(sender_gen) => SessionCleanup::attached(state, cleanup_session_id, sender_gen),
        };
        let endpoint_uri = format!("?session_id={}", session_id);
        yield Event::message(endpoint_uri).event_type("endpoint");

        while let Some(msg) = rx.recv().await {
             yield Event::message(msg).event_type("message");
        }
    })
    .keep_alive(SSE_KEEP_ALIVE_INTERVAL)
    .into_response())
}

#[handler]
async fn post_handler<ToolsType, PromptsType, ResourcesType>(
    data: Data<&Arc<State<ToolsType, PromptsType, ResourcesType>>>,
    request: &Request,
    batch_request: Json<McpBatchRequest>,
    accept: Accept,
    query: Query<HashMap<String, String>>,
) -> poem::Result<Response>
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
            return Ok(StatusCode::BAD_REQUEST.into_response());
        };

        if batch_request.len() == 1 && batch_request.requests()[0].is_initialize() {
            let mut server = data.0.make_server(request)?;
            let session_id = session_id();
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
                    legacy_sse: false,
                    sender: None,
                    sender_gen: 0,
                    last_active: Instant::now(),
                },
            );

            tracing::info!(session_id, "created new streamable HTTP session");
            return Ok(Json(resp)
                .with_header("Mcp-Session-Id", session_id)
                .into_response());
        }

        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let session_id = session_id_param.unwrap();

    let (server, sender) = {
        let mut sessions = data.0.sessions.lock().unwrap();
        let Some(session) = sessions.get_mut(&session_id) else {
            tracing::warn!(session_id, "session not found (expired or invalid)");
            return Ok(StatusCode::NOT_FOUND.into_response());
        };
        session.last_active = Instant::now();
        (
            session.server.clone(),
            session.sender.clone().filter(|_| session.legacy_sse),
        )
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
                    return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
            }
        }
        return Ok(StatusCode::ACCEPTED.into_response());
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

    Ok(match accept {
        "text/event-stream" => {
            if all_notifications {
                return Ok(StatusCode::ACCEPTED.into_response());
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
                return Ok(StatusCode::ACCEPTED.into_response());
            }
            Json(resps)
                .with_content_type("application/json")
                .into_response()
        }
    })
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
///
/// The factory may return an [`McpServer`] directly or `Result<McpServer, E>`
/// where `E: Into<poem::Error>`. A factory error is propagated through Poem's
/// normal error handling without creating a session or caching server metadata.
///
/// The factory runs only when creating a new session: a POST `initialize`
/// without a session ID, or a legacy SSE GET without a session ID. Requests for
/// an existing session do not run it again. Use middleware to authenticate and
/// authorize every request, including access to existing session IDs.
///
/// # Fallible factory
///
/// This example reads a user already authenticated by middleware from request
/// data. Failure to provide that data rejects session creation.
///
/// ```rust,no_run
/// use poem::{Route, http::StatusCode};
/// use poem_mcpserver::{McpServer, streamable_http};
///
/// struct AuthenticatedUser {
///     name: String,
/// }
///
/// let app = Route::new().at(
///     "/mcp",
///     streamable_http::endpoint(|request| {
///         let user = request
///             .data::<AuthenticatedUser>()
///             .ok_or(StatusCode::UNAUTHORIZED)?;
///         Ok::<_, poem::Error>(McpServer::new().with_server_info(&user.name, "1.0"))
///     }),
/// );
/// ```
pub fn endpoint<F, ToolsType, PromptsType, ResourcesType>(server_factory: F) -> impl IntoEndpoint
where
    F: McpServerFactory<ToolsType, PromptsType, ResourcesType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    endpoint_with_config(server_factory, Config::default())
}

/// A streamable http endpoint with configurable session behavior.
///
/// Accepts the same infallible or fallible factories as [`endpoint`]. Factory
/// errors propagate through Poem's normal error handling without creating a
/// session or caching metadata. The factory runs only for new sessions; use
/// middleware for per-request authentication and authorization of session IDs.
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
/// (server info, registered resources, disabled tools, ...) is shared via an
/// [`Arc`] when it is identical to the first successful factory result. This
/// keeps the per-session memory footprint small for constant configuration
/// while preserving request-specific metadata. The per-session mutable state of
/// your `Tools` / `Prompts` / `Resources` implementations is still produced
/// fresh by the factory on every new session.
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
    F: McpServerFactory<ToolsType, PromptsType, ResourcesType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
    PromptsType: Prompts + Send + Sync + 'static,
    ResourcesType: Resources + Send + Sync + 'static,
{
    let state = Arc::new(State {
        server_factory: Box::new(move |request| server_factory(request).into_mcp_server()),
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
    use std::{
        sync::{Arc, OnceLock},
        time::Duration,
    };

    use poem::{
        EndpointExt,
        http::{Method, StatusCode},
        post,
        test::TestClient,
    };
    use serde_json::json;
    use tokio_stream::StreamExt;

    use super::{Config, State, endpoint_with_config, get_handler, post_handler};
    use crate::{McpServer, prompts::NoPrompts, resources::NoResources, tool::NoTools};

    #[test]
    fn metadata_is_shared_only_when_all_contents_match() {
        let state = State {
            server_factory: Box::new(|request: &poem::Request| {
                let change = request
                    .headers()
                    .get("x-config")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default();
                let server = McpServer::new().ui_resource(
                    "ui://account",
                    if change == "resource" {
                        "Different name"
                    } else {
                        "Account"
                    },
                    "Account UI",
                    "text/html",
                    if change == "content" {
                        "private"
                    } else {
                        "shared"
                    },
                );
                Ok(match change {
                    "info" => server.with_server_info("different", "1"),
                    "tools" => server.disable_tools(["private_tool"]),
                    _ => server,
                })
            }),
            sessions: Default::default(),
            shared_metadata: OnceLock::new(),
        };
        let first = state.make_server(&poem::Request::default()).unwrap();
        let second = state.make_server(&poem::Request::default()).unwrap();
        assert!(Arc::ptr_eq(first.metadata(), second.metadata()));

        for change in ["content", "resource", "info", "tools"] {
            let different = state
                .make_server(&poem::Request::builder().header("x-config", change).finish())
                .unwrap();
            assert!(
                !Arc::ptr_eq(first.metadata(), different.metadata()),
                "must preserve differing {change}"
            );
        }
        let same_again = state.make_server(&poem::Request::default()).unwrap();
        assert!(Arc::ptr_eq(first.metadata(), same_again.metadata()));
    }

    #[tokio::test]
    async fn failed_factory_does_not_insert_sessions_or_cache_metadata() {
        let state = Arc::new(State {
            server_factory: Box::new(|request: &poem::Request| {
                if !request.headers().contains_key("x-allow-session") {
                    return Err(StatusCode::FORBIDDEN.into());
                }
                Ok(McpServer::new().with_server_info("successful", "1"))
            }),
            sessions: Default::default(),
            shared_metadata: OnceLock::new(),
        });
        let client = TestClient::new(
            post(post_handler::<NoTools, NoPrompts, NoResources>::default())
                .get(get_handler::<NoTools, NoPrompts, NoResources>::default())
                .data(state.clone()),
        );
        let initialize = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26", "capabilities": {},
                "clientInfo": {"name": "test", "version": "1"}
            }
        });
        for method in [Method::GET, Method::POST] {
            client
                .request(method, "/")
                .header("Accept", "application/json")
                .body_json(&initialize)
                .send()
                .await
                .assert_status(StatusCode::FORBIDDEN);
            assert!(state.sessions.lock().unwrap().is_empty());
            assert!(state.shared_metadata.get().is_none());
        }
        client
            .post("/")
            .header("Accept", "application/json")
            .header("x-allow-session", "yes")
            .body_json(&initialize)
            .send()
            .await
            .assert_status_is_ok();
        assert_eq!(state.sessions.lock().unwrap().len(), 1);
        let metadata = state.shared_metadata.get().unwrap().clone();
        assert_eq!(metadata.server_info.name, "successful");

        // Later failures must leave existing sessions and the shared cache
        // intact.
        for method in [Method::GET, Method::POST] {
            client
                .request(method, "/")
                .header("Accept", "application/json")
                .body_json(&initialize)
                .send()
                .await
                .assert_status(StatusCode::FORBIDDEN);
            assert_eq!(state.sessions.lock().unwrap().len(), 1);
            assert!(Arc::ptr_eq(&metadata, state.shared_metadata.get().unwrap()));
        }
    }

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
