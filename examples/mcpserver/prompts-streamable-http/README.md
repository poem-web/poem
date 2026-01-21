# prompts-streamable-http

This example runs a streamable HTTP MCP server with tools and prompts, plus a small rmcp client that connects to it.

## Run the server

1. From the repo root, in one terminal:
   cargo run --manifest-path ./examples/mcpserver/prompts-streamable-http/Cargo.toml

   Or from the example directory:
   cargo run

The server listens on http://127.0.0.1:8000/.

## Run the client (rmcp)

2. From the repo root, in another terminal:
   cargo run --manifest-path ./examples/mcpserver/prompts-streamable-http/Cargo.toml --bin prompts-streamable-http-client

   Or from the example directory:
   cargo run --bin prompts-streamable-http-client

The client lists tools and invokes the get_review_count tool.

## Tip

Because this example is not a workspace member, run commands from this directory or use --manifest-path from the repo root.
