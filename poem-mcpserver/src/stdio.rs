//! Standard input and output server implementation.

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{
    protocol::{
        rpc::{Request, Response, RpcError},
        JSON_RPC_VERSION,
    },
    tool::Tools,
    McpServer,
};

fn print_response(response: impl Serialize) {
    println!("{}", serde_json::to_string(&response).unwrap());
}

/// Run the server using standard input and output.
pub async fn stdio<ToolsType>(server: McpServer<ToolsType>) -> std::io::Result<()>
where
    ToolsType: Tools,
{
    let mut server = server;
    let mut input = BufReader::new(tokio::io::stdin()).lines();

    while let Some(line) = input.next_line().await? {
        tracing::info!(request = &line, "received request");

        let Ok(request) = serde_json::from_str::<Request>(&line) else {
            continue;
        };

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
            tracing::info!(response = ?resp, "sending response");
            print_response(resp);
        }
    }

    Ok(())
}
