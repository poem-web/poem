//! Server-sent events endpoint for handling MCP requests.

use std::{collections::HashMap, sync::Arc};

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
use tokio::sync::{mpsc::Sender, Mutex};

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
    let connections = data.connections.lock().await;
    let Some(sender) = connections.get(&session_id) else {
        return StatusCode::BAD_REQUEST;
    };
    if sender.send(request.0).await.is_err() {
        return StatusCode::BAD_REQUEST;
    }
    StatusCode::OK
}

#[handler]
async fn events_handler<ToolsType>(data: Data<&Arc<State<ToolsType>>>) -> impl IntoResponse
where
    ToolsType: Tools + Send + Sync + 'static,
{
    let session_id = session_id();
    let mut server = (data.server_factory)();
    let state = data.0.clone();
    let mut connections = data.connections.lock().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    connections.insert(session_id.clone(), tx);

    SSE::new(async_stream::stream! {
        yield Event::message(format!("?session_id={}", session_id)).event_type("endpoint");
        while let Some(req) = rx.recv().await {
            if let Some(resp) = server.handle_request(req).await {
                yield Event::message(serde_json::to_string(&resp).unwrap()).event_type("message");
            }
        }
        state.connections.lock().await.remove(&session_id);
    })
}

/// A server-sent events endpoint that can be used to handle MCP requests.
pub fn sse_endpoint<F, ToolsType>(server_factory: F) -> impl IntoEndpoint
where
    F: Fn() -> McpServer<ToolsType> + Send + Sync + 'static,
    ToolsType: Tools + 'static,
{
    get(events_handler::<ToolsType>::default())
        .post(post_handler::<ToolsType>::default())
        .data(Arc::new(State {
            server_factory: Box::new(server_factory),
            connections: Default::default(),
        }))
}
