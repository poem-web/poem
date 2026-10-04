use poem_mcpserver::{
    protocol::{
        resources::{
            Resource, ResourceContent, ResourceTemplate, ResourcesListRequest,
            ResourcesListResponse, ResourcesReadRequest, ResourcesReadResponse,
            ResourcesTemplatesListRequest, ResourcesTemplatesListResponse,
        },
        rpc::RpcError,
    },
    resources::Resources,
    stdio::stdio,
    McpServer,
};

/// A dynamic resource provider that serves config and status info.
struct ConfigResources;

impl Resources for ConfigResources {
    async fn list(
        &self,
        _request: ResourcesListRequest,
    ) -> Result<ResourcesListResponse, RpcError> {
        Ok(ResourcesListResponse {
            resources: vec![
                Resource {
                    uri: "config://app/settings".to_string(),
                    name: "Application Settings".to_string(),
                    description: "Current application configuration".to_string(),
                    mime_type: "application/json".to_string(),
                },
                Resource {
                    uri: "config://app/status".to_string(),
                    name: "Application Status".to_string(),
                    description: "Current application runtime status".to_string(),
                    mime_type: "text/plain".to_string(),
                },
            ],
        })
    }

    async fn templates(
        &self,
        _request: ResourcesTemplatesListRequest,
    ) -> Result<ResourcesTemplatesListResponse, RpcError> {
        Ok(ResourcesTemplatesListResponse {
            resource_templates: vec![ResourceTemplate {
                uri_template: "config://app/env/{name}".to_string(),
                name: "Environment Variable".to_string(),
                description: "Read a specific environment variable".to_string(),
                mime_type: "text/plain".to_string(),
            }],
        })
    }

    async fn read(&self, request: ResourcesReadRequest) -> Result<ResourcesReadResponse, RpcError> {
        let content = match request.uri.as_str() {
            "config://app/settings" => ResourceContent {
                uri: request.uri,
                mime_type: "application/json".to_string(),
                text: Some(
                    r#"{"debug": false, "log_level": "info", "max_connections": 100}"#.to_string(),
                ),
                blob: None,
            },
            "config://app/status" => ResourceContent {
                uri: request.uri,
                mime_type: "text/plain".to_string(),
                text: Some("Running - uptime: 42s".to_string()),
                blob: None,
            },
            uri if uri.starts_with("config://app/env/") => {
                let var_name = uri.strip_prefix("config://app/env/").unwrap();
                let value = std::env::var(var_name).unwrap_or_else(|_| "(not set)".to_string());
                ResourceContent {
                    uri: request.uri,
                    mime_type: "text/plain".to_string(),
                    text: Some(value),
                    blob: None,
                }
            }
            _ => {
                return Err(RpcError::invalid_params(format!(
                    "resource not found: {}",
                    request.uri
                )));
            }
        };

        Ok(ResourcesReadResponse {
            contents: vec![content],
        })
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    stdio(McpServer::new().resources(ConfigResources).ui_resource(
        "static://readme",
        "README",
        "A static readme resource",
        "text/plain",
        "Welcome to the resources example!",
    ))
    .await
}
