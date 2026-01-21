use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam, service::ServiceExt, transport::StreamableHttpClientTransport,
};

#[tokio::main]
async fn main() -> Result<()> {
    let transport = StreamableHttpClientTransport::from_uri("http://127.0.0.1:8000/");
    let service = ().serve(transport).await?;

    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {tools:#?}");

    // Sending empty arguments object for get_review_count tool to satisfy schema

    let response = service
        .call_tool(CallToolRequestParam {
            name: "get_review_count".to_string().into(),
            arguments: Some(serde_json::Map::new()),
            task: None,
        })
        .await?;
    println!("get_review_count response: {response:#?}");

    service.cancel().await?;
    Ok(())
}
