//! Server-sent events endpoint for handling MCP requests.

use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use poem::{
    get, handler,
    http::StatusCode,
    web::{
        sse::{Event, SSE},
        Data, Json, Query,
    },
    EndpointExt, IntoEndpoint, IntoResponse,
};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_stream::Stream;

use crate::{protocol::rpc::Request as McpRequest, tool::Tools, McpServer};

struct State<ToolsType> {
    server_factory: Box<dyn Fn() -> McpServer<ToolsType> + Send + Sync>,
    connections: Mutex<HashMap<String, Sender<McpRequest>>>,
}

fn session_id() -> String {
    format!("{:016x}", rand::random::<u128>())
}

#[derive(Debug, Deserialize)]
struct PostQuery {
    session_id: String,
}

#[handler]
async fn post_handler<ToolsType>(
    Query(PostQuery { session_id }): Query<PostQuery>,
    request: Json<McpRequest>,
    data: Data<&Arc<State<ToolsType>>>,
) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
{
    let connections = data.connections.lock().unwrap().clone();
    let Some(sender) = connections.get(&session_id) else {
        return StatusCode::NOT_FOUND;
    };
    _ = sender.send(request.0).await;
    StatusCode::OK
}

pin_project! {
    struct SseStream<S, ToolsType> {
        #[pin]
        inner: S,
        session_id: String,
        state: Arc<State<ToolsType>>,
    }

    impl<S, ToolsType> PinnedDrop for SseStream<S, ToolsType> {
        fn drop(this: Pin<&mut Self>) {
            this.state.connections.lock().unwrap().remove(&this.session_id);
            tracing::info!(session_id = this.session_id, "mcp connection closed");
        }
    }
}

impl<S, ToolsType> Stream for SseStream<S, ToolsType>
where
    S: Stream<Item = Event> + Send + 'static,
    ToolsType: Tools + Send + Sync + 'static,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}

#[handler]
async fn events_handler<ToolsType>(data: Data<&Arc<State<ToolsType>>>) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
{
    let session_id = session_id();
    let mut server = (data.server_factory)();
    let state = data.0.clone();
    let mut connections = data.connections.lock().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    tracing::info!(session_id = session_id, "new mcp connection");
    connections.insert(session_id.clone(), tx);

    SSE::new(SseStream {
        inner: {
            let session_id = session_id.clone();
            async_stream::stream! {
                yield Event::message(format!("?session_id={}", session_id)).event_type("endpoint");
                while let Some(req) = rx.recv().await {
                    tracing::info!(session_id = session_id, request = ?req, "received request");
                    if let Some(resp) = server.handle_request(req).await {
                        tracing::info!(session_id = session_id, response = ?resp, "sending response");
                        yield Event::message(serde_json::to_string(&resp).unwrap()).event_type("message");
                    }
                }
            }
        },
        session_id: session_id.clone(),
        state: state.clone(),
    })
}

/// A server-sent events endpoint that can be used to handle MCP requests.
pub fn sse_endpoint<F, ToolsType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn() -> McpServer<ToolsType> + Send + Sync + 'static,
    ToolsType: Tools + Send + Sync + 'static,
{
    get(events_handler::<ToolsType>::default())
        .post(post_handler::<ToolsType>::default())
        .data(Arc::new(State {
            server_factory: Box::new(server_factory),
            connections: Default::default(),
        }))
}
