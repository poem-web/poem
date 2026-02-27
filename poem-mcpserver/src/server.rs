use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::{
    prompts::{NoPrompts, Prompts},
    protocol::{
        JSON_RPC_VERSION,
        initialize::{
            InitializeRequest, InitializeResponse, PromptsCapability, ResourcesCapability,
            ServerCapabilities, ServerInfo, ToolsCapability,
        },
        prompts::{PromptsGetRequest, PromptsListResponse},
        resources::{
            Resource, ResourceContent, ResourcesListResponse, ResourcesReadRequest,
            ResourcesReadResponse, ResourcesTemplatesListResponse,
        },
        rpc::{Request, RequestId, Requests, Response},
        tool::{ToolsCallRequest, ToolsListResponse},
    },
    tool::{NoTools, Tools},
};

/// A server that can be used to handle MCP requests.
pub struct McpServer<ToolsType = NoTools, PromptsType = NoPrompts> {
    tools: ToolsType,
    prompts: PromptsType,
    disabled_tools: HashSet<String>,
    server_info: ServerInfo,
    resources: Vec<Resource>,
    resource_contents: HashMap<String, ResourceContent>,
}

impl Default for McpServer<NoTools, NoPrompts> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer<NoTools, NoPrompts> {
    /// Creates a new MCP server.
    #[inline]
    pub fn new() -> Self {
        Self {
            tools: NoTools,
            prompts: NoPrompts,
            disabled_tools: HashSet::new(),
            server_info: ServerInfo {
                name: "poem-mcpserver".to_string(),
                version: "0.1.0".to_string(),
            },
            resources: Vec::new(),
            resource_contents: HashMap::new(),
        }
    }
}

impl<ToolsType, PromptsType> McpServer<ToolsType, PromptsType>
where
    ToolsType: Tools,
    PromptsType: Prompts,
{
    /// Sets the tools that the server will use.
    #[inline]
    pub fn tools<T>(self, tools: T) -> McpServer<T, PromptsType>
    where
        T: Tools,
    {
        McpServer {
            tools,
            prompts: self.prompts,
            disabled_tools: self.disabled_tools,
            server_info: self.server_info,
            resources: self.resources,
            resource_contents: self.resource_contents,
        }
    }

    /// Sets the prompts that the server will use.
    #[inline]
    pub fn prompts<P>(self, prompts: P) -> McpServer<ToolsType, P>
    where
        P: Prompts,
    {
        McpServer {
            tools: self.tools,
            prompts,
            disabled_tools: self.disabled_tools,
            server_info: self.server_info,
            resources: self.resources,
            resource_contents: self.resource_contents,
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

    /// Adds a static UI resource.
    pub fn ui_resource(
        mut self,
        uri: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        mime_type: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let uri = uri.into();
        let resource = Resource {
            uri: uri.clone(),
            name: name.into(),
            description: description.into(),
            mime_type: mime_type.into(),
        };
        let content = ResourceContent {
            uri: uri.clone(),
            mime_type: resource.mime_type.clone(),
            text: Some(text.into()),
            blob: None,
        };
        self.resources.push(resource);
        self.resource_contents.insert(uri, content);
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

    fn handle_prompts_list(&self, id: Option<RequestId>) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(PromptsListResponse {
                prompts: PromptsType::list(),
            }),
            error: None,
        }
        .map_result_to_value()
    }

    async fn handle_prompts_get(
        &self,
        request: PromptsGetRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.prompts.get(&request.name, request.arguments).await {
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

    fn handle_resources_list(&self, id: Option<RequestId>) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(ResourcesListResponse {
                resources: self.resources.clone(),
            }),
            error: None,
        }
        .map_result_to_value()
    }

    fn handle_resources_templates_list(&self, id: Option<RequestId>) -> Response<Value> {
        Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result: Some(ResourcesTemplatesListResponse {
                resource_templates: vec![],
            }),
            error: None,
        }
        .map_result_to_value()
    }

    fn handle_resources_read(
        &self,
        request: ResourcesReadRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.resource_contents.get(&request.uri) {
            Some(content) => Response {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: Some(ResourcesReadResponse {
                    contents: vec![content.clone()],
                }),
                error: None,
            }
            .map_result_to_value(),
            None => Response::<()> {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(crate::protocol::rpc::RpcError::invalid_params(format!(
                    "resource not found: {}",
                    request.uri
                ))),
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
            Requests::PromptsList { .. } => Some(self.handle_prompts_list(request.id)),
            Requests::PromptsGet { params } => {
                Some(self.handle_prompts_get(params, request.id).await)
            }
            Requests::ResourcesList { .. } => Some(self.handle_resources_list(request.id)),
            Requests::ResourcesTemplatesList { .. } => {
                Some(self.handle_resources_templates_list(request.id))
            }
            Requests::ResourcesRead { params } => {
                Some(self.handle_resources_read(params, request.id))
            }
        }
    }
}
