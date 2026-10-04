//! Standard input and output server implementation.

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{
    McpServer,
    prompts::Prompts,
    protocol::{
        JSON_RPC_VERSION,
        rpc::{BatchRequest, Response, RpcError},
    },
    resources::Resources,
    tool::Tools,
};

const REQUEST_LOG_TARGET: &str = "poem_mcpserver::payload::request";
const RESPONSE_LOG_TARGET: &str = "poem_mcpserver::payload::response";

fn print_response(response: impl Serialize) {
    println!("{}", serde_json::to_string(&response).unwrap());
}

/// Run the server using standard input and output.
pub async fn stdio<ToolsType, PromptsType, ResourcesType>(
    server: McpServer<ToolsType, PromptsType, ResourcesType>,
) -> std::io::Result<()>
where
    ToolsType: Tools,
    PromptsType: Prompts,
    ResourcesType: Resources,
{
    let mut server = server;
    let mut input = BufReader::new(tokio::io::stdin()).lines();

    tracing::info!("stdio server started");

    while let Some(line) = input.next_line().await? {
        tracing::info!(target: REQUEST_LOG_TARGET, request = &line, "received request");

        let Ok(batch_request) = serde_json::from_str::<BatchRequest>(&line).inspect_err(|err| {
            tracing::error!(error = ?err, "failed to parse request");
        }) else {
            continue;
        };

        for request in batch_request.into_iter() {
            if request.jsonrpc != JSON_RPC_VERSION {
                print_response(Response::<()> {
                    jsonrpc: JSON_RPC_VERSION.to_string(),
                    id: request.id,
                    result: None,
                    error: Some(RpcError::invalid_request(
                        "invalid JSON-RPC version, expected `2.0`",
                    )),
                });
                continue;
            }

            if let Some(resp) = server.handle_request(request).await {
                tracing::info!(target: RESPONSE_LOG_TARGET, response = ?resp, "sending response");
                print_response(resp);
            }
        }
    }

    Ok(())
}
