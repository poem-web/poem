use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
            Resource, ResourceContent, ResourcesListRequest, ResourcesReadRequest,
            ResourcesReadResponse, ResourcesTemplatesListRequest,
        },
        rpc::{Request, RequestId, Requests, Response},
        tool::{ToolsCallRequest, ToolsListResponse},
    },
    resources::{NoResources, Resources},
    tool::{NoTools, Tools, normalize_schema_value},
};

/// Shared, immutable metadata for an [`McpServer`].
///
/// These fields are configured once via the builder methods on [`McpServer`]
/// and remain stable for the entire lifetime of the server, so they are kept
/// behind an [`Arc`] and shared across all sessions to keep the per-session
/// footprint small.
#[derive(Clone)]
pub(crate) struct ServerMetadata {
    pub disabled_tools: HashSet<String>,
    pub server_info: ServerInfo,
    pub resources: Vec<Resource>,
    pub resource_contents: HashMap<String, ResourceContent>,
}

impl ServerMetadata {
    fn new() -> Self {
        Self {
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

/// A server that can be used to handle MCP requests.
pub struct McpServer<ToolsType = NoTools, PromptsType = NoPrompts, ResourcesType = NoResources> {
    tools: ToolsType,
    prompts: PromptsType,
    resources_handler: ResourcesType,
    meta: Arc<ServerMetadata>,
}

impl Default for McpServer<NoTools, NoPrompts, NoResources> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer<NoTools, NoPrompts, NoResources> {
    /// Creates a new MCP server.
    #[inline]
    pub fn new() -> Self {
        Self {
            tools: NoTools,
            prompts: NoPrompts,
            resources_handler: NoResources,
            meta: Arc::new(ServerMetadata::new()),
        }
    }
}

impl<ToolsType, PromptsType, ResourcesType> McpServer<ToolsType, PromptsType, ResourcesType>
where
    ToolsType: Tools,
    PromptsType: Prompts,
    ResourcesType: Resources,
{
    /// Sets the tools that the server will use.
    #[inline]
    pub fn tools<T>(self, tools: T) -> McpServer<T, PromptsType, ResourcesType>
    where
        T: Tools,
    {
        McpServer {
            tools,
            prompts: self.prompts,
            resources_handler: self.resources_handler,
            meta: self.meta,
        }
    }

    /// Sets the prompts that the server will use.
    #[inline]
    pub fn prompts<P>(self, prompts: P) -> McpServer<ToolsType, P, ResourcesType>
    where
        P: Prompts,
    {
        McpServer {
            tools: self.tools,
            prompts,
            resources_handler: self.resources_handler,
            meta: self.meta,
        }
    }

    /// Sets the resources that the server will use.
    #[inline]
    pub fn resources<R>(self, resources_handler: R) -> McpServer<ToolsType, PromptsType, R>
    where
        R: Resources,
    {
        McpServer {
            tools: self.tools,
            prompts: self.prompts,
            resources_handler,
            meta: self.meta,
        }
    }

    /// Disables tools by their names.
    pub fn disable_tools<I, T>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Arc::make_mut(&mut self.meta)
            .disabled_tools
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
        let meta = Arc::make_mut(&mut self.meta);
        meta.resources.push(resource);
        meta.resource_contents.insert(uri, content);
        self
    }

    /// Sets the server info (name and version).
    pub fn with_server_info(mut self, name: &str, version: &str) -> Self {
        Arc::make_mut(&mut self.meta).server_info = ServerInfo {
            name: name.to_string(),
            version: version.to_string(),
        };
        self
    }

    /// Returns the shared metadata of this server.
    ///
    /// Used by transports (e.g. `streamable_http`) to deduplicate the
    /// configuration across sessions.
    #[cfg(feature = "streamable-http")]
    #[inline]
    pub(crate) fn metadata(&self) -> &Arc<ServerMetadata> {
        &self.meta
    }

    /// Replaces the shared metadata of this server.
    ///
    /// Used by transports to attach a cached, shared metadata instance to a
    /// freshly produced server, so that the per-session footprint stays small.
    #[cfg(feature = "streamable-http")]
    #[inline]
    pub(crate) fn set_metadata(&mut self, meta: Arc<ServerMetadata>) {
        self.meta = meta;
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
                server_info: self.meta.server_info.clone(),
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
                    tools.retain(|tool| !self.meta.disabled_tools.contains(tool.name));

                    for tool in &mut tools {
                        tool.input_schema =
                            normalize_schema_value(std::mem::take(&mut tool.input_schema));
                        tool.output_schema = tool.output_schema.take().map(normalize_schema_value);

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

    async fn handle_resources_list(
        &self,
        request: ResourcesListRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.resources_handler.list(request).await {
            Ok(mut response) => {
                response
                    .resources
                    .extend(self.meta.resources.iter().cloned());
                Response {
                    jsonrpc: JSON_RPC_VERSION.to_string(),
                    id,
                    result: Some(response),
                    error: None,
                }
                .map_result_to_value()
            }
            Err(err) => Response::<()> {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(err),
            }
            .map_result_to_value(),
        }
    }

    async fn handle_resources_templates_list(
        &self,
        request: ResourcesTemplatesListRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.resources_handler.templates(request).await {
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

    async fn handle_resources_read(
        &self,
        request: ResourcesReadRequest,
        id: Option<RequestId>,
    ) -> Response<Value> {
        match self.meta.resource_contents.get(&request.uri) {
            Some(content) => Response {
                jsonrpc: JSON_RPC_VERSION.to_string(),
                id,
                result: Some(ResourcesReadResponse {
                    contents: vec![content.clone()],
                }),
                error: None,
            }
            .map_result_to_value(),
            None => match self.resources_handler.read(request).await {
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
            },
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
            Requests::ResourcesList { params } => {
                Some(self.handle_resources_list(params, request.id).await)
            }
            Requests::ResourcesTemplatesList { params } => Some(
                self.handle_resources_templates_list(params, request.id)
                    .await,
            ),
            Requests::ResourcesRead { params } => {
                Some(self.handle_resources_read(params, request.id).await)
            }
        }
    }
}
