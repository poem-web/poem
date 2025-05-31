use std::collections::HashSet;

use serde_json::Value;

use crate::{
    protocol::{
        JSON_RPC_VERSION,
        initialize::{
            InitializeRequest, InitializeResponse, PromptsCapability, ResourcesCapability,
            ServerCapabilities, ServerInfo, ToolsCapability,
        },
        prompts::PromptsListResponse,
        resources::ResourcesListResponse,
        rpc::{Request, RequestId, Requests, Response},
        tool::{ToolsCallRequest, ToolsListResponse},
    },
    tool::{NoTools, Tools},
};

/// A server that can be used to handle MCP requests.
pub struct McpServer<ToolsType = NoTools> {
    tools: ToolsType,
    disabled_tools: HashSet<String>,
    server_info: ServerInfo,
}

impl Default for McpServer<NoTools> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer<NoTools> {
    /// Creates a new MCP server.
    #[inline]
    pub fn new() -> Self {
        Self {
            tools: NoTools,
            disabled_tools: HashSet::new(),
            server_info: ServerInfo {
                name: "poem-mcpserver".to_string(),
                version: "0.1.0".to_string(),
            },
        }
    }
}

impl<ToolsType> McpServer<ToolsType>
where
    ToolsType: Tools,
{
    /// Sets the tools that the server will use.
    #[inline]
    pub fn tools<T>(self, tools: T) -> McpServer<T>
    where
        T: Tools,
    {
        McpServer {
            tools,
            disabled_tools: self.disabled_tools,
            server_info: self.server_info,
        }
    }

    /// Disables tools by their names.
    pub fn disable_tools<I, T>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.disabled_tools
            .extend(names.into_iter().map(Into::into));
        self
    }

    /// Sets the server info (name and version).
    pub fn with_server_info(mut self, name: &str, version: &str) -> Self {
        self.server_info = ServerInfo {
            name: name.to_string(),
            version: version.to_string(),
        };
        self
    }

    fn handle_ping(&self, id: Option<RequestId>) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(Value::Object(Default::default())),
            error: None,
        }
    }

    fn handle_initialize(
        &self,
        request: InitializeRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(InitializeResponse {
                protocol_version: request.protocol_version,
                capabilities: ServerCapabilities {
                    prompts: PromptsCapability {
                        list_changed: false,
                    },
                    resources: ResourcesCapability {
                        list_changed: false,
                        subscribe: false,
                    },
                    tools: ToolsCapability {
                        list_changed: false,
                    },
                },
                server_info: self.server_info.clone(),
                instructions: Some(ToolsType::instructions().to_string()),
            }),
            error: None,
        }
        .map_result_to_value()
    }

    fn handle_tools_list(&self, id: Option<RequestId>) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(ToolsListResponse {
                tools: {
                    let mut tools = ToolsType::list();
                    tools.retain(|tool| !self.disabled_tools.contains(tool.name));

                    for tool in &mut tools {
                        if let Some(object) = tool.input_schema.as_object_mut() {
                            if !object.contains_key("properties") {
                                object.insert(
                                    "properties".to_string(),
                                    Value::Object(Default::default()),
                                );
                            }
                        }
                    }
                    tools
                },
            }),
            error: None,
        }
        .map_result_to_value()
    }

    async fn handle_tools_call(
        &mut self,
        request: ToolsCallRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.tools.call(&request.name, request.arguments).await {
            Ok(response) => Response {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: Some(response),
                error: None,
            }
            .map_result_to_value(),
            Err(err) => Response::<()> {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(err),
            }
            .map_result_to_value(),
        }
    }

    /// Handles a request and returns a response.
    pub async fn handle_request(&mut self, request: Request) -> Option<Response<Value>> {
        match request.body {
            Requests::Ping => Some(self.handle_ping(request.id)),
            Requests::Initialize { params } => Some(self.handle_initialize(params, request.id)),
            Requests::Initialized => None,
            Requests::Cancelled { .. } => None,
            Requests::ToolsList { .. } => Some(self.handle_tools_list(request.id)),
            Requests::ToolsCall { params } => {
                Some(self.handle_tools_call(params, request.id).await)
            }
            Requests::PromptsList { .. } => Some(
                Response {
                    jsonrpc: JSON_RPC_VERSION.to_string(),
                    id: request.id,
                    result: Some(PromptsListResponse { prompts: vec![] }),
                    error: None,
                }
                .map_result_to_value(),
            ),
            Requests::ResourcesList { .. } => Some(
                Response {
                    jsonrpc: JSON_RPC_VERSION.to_string(),
                    id: request.id,
                    result: Some(ResourcesListResponse { resources: vec![] }),
                    error: None,
                }
                .map_result_to_value(),
            ),
        }
    }
}
