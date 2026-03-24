use anyhow::Result;
use rmcp::{
    model::CallToolRequestParams, service::ServiceExt, transport::StreamableHttpClientTransport,
};

#[tokio::main]
async fn main() -> Result<()> {
    let transport = StreamableHttpClientTransport::from_uri("http://127.0.0.1:8000/");
    let service = ().serve(transport).await?;

    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {tools:#?}");

    // Sending empty arguments object for get_review_count tool to satisfy schema

    let response = service
        .call_tool(
            CallToolRequestParams::new("get_review_count").with_arguments(serde_json::Map::new()),
        )
        .await?;
    println!("get_review_count response: {response:#?}");

    service.cancel().await?;
    Ok(())
}
