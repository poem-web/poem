use std::collections::HashMap;

use poem_mcpserver::{
    McpServer, Prompts,
    content::Text,
    prompts::PromptMessages,
    protocol::{
        JSON_RPC_VERSION,
        prompts::{PromptsGetRequest, PromptsListRequest},
        rpc::{Request, RequestId, Requests},
    },
};

struct TestPrompts {
    system_name: String,
}

impl TestPrompts {
    fn new() -> Self {
        Self {
            system_name: "TestSystem".to_string(),
        }
    }
}

#[Prompts]
impl TestPrompts {
    /// A simple greeting prompt.
    async fn greet(
        &self,
        /// The name to greet
        #[mcp(required)]
        name: Option<String>,
    ) -> String {
        format!("Hello, {}! Welcome to {}.", name.unwrap(), self.system_name)
    }

    /// A code review prompt with optional language parameter.
    async fn code_review(
        &self,
        /// The code to review
        #[mcp(required)]
        code: Option<String>,
        /// The programming language
        language: Option<String>,
    ) -> PromptMessages {
        let lang = language.unwrap_or_else(|| "unknown".to_string());
        PromptMessages::new()
            .user(Text(format!(
                "Please review the following {} code:\n\n```{}\n{}\n```",
                lang,
                lang,
                code.unwrap()
            )))
            .assistant(Text("I'll review this code for you. Let me analyze it...".to_string()))
    }

    /// A simple prompt without required arguments.
    async fn help(&self) -> String {
        "How can I help you today?".to_string()
    }
}

#[tokio::test]
async fn prompts_list() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(1)),
            body: Requests::PromptsList {
                params: PromptsListRequest { cursor: None },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "prompts": [
                    {
                        "name": "greet",
                        "description": "A simple greeting prompt.",
                        "arguments": [
                            {
                                "name": "name",
                                "description": "The name to greet",
                                "required": true
                            }
                        ]
                    },
                    {
                        "name": "code_review",
                        "description": "A code review prompt with optional language parameter.",
                        "arguments": [
                            {
                                "name": "code",
                                "description": "The code to review",
                                "required": true
                            },
                            {
                                "name": "language",
                                "description": "The programming language",
                                "required": false
                            }
                        ]
                    },
                    {
                        "name": "help",
                        "description": "A simple prompt without required arguments.",
                        "arguments": []
                    }
                ]
            }
        })
    );
}

#[tokio::test]
async fn prompts_get_simple() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let mut arguments = HashMap::new();
    arguments.insert("name".to_string(), "Alice".to_string());

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(2)),
            body: Requests::PromptsGet {
                params: PromptsGetRequest {
                    name: "greet".to_string(),
                    arguments,
                },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "description": "",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Hello, Alice! Welcome to TestSystem."
                        }
                    }
                ]
            }
        })
    );
}

#[tokio::test]
async fn prompts_get_with_multiple_messages() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let mut arguments = HashMap::new();
    arguments.insert("code".to_string(), "fn main() {}".to_string());
    arguments.insert("language".to_string(), "rust".to_string());

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(3)),
            body: Requests::PromptsGet {
                params: PromptsGetRequest {
                    name: "code_review".to_string(),
                    arguments,
                },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "description": "",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Please review the following rust code:\n\n```rust\nfn main() {}\n```"
                        }
                    },
                    {
                        "role": "assistant",
                        "content": {
                            "type": "text",
                            "text": "I'll review this code for you. Let me analyze it..."
                        }
                    }
                ]
            }
        })
    );
}

#[tokio::test]
async fn prompts_get_missing_required_argument() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(4)),
            body: Requests::PromptsGet {
                params: PromptsGetRequest {
                    name: "greet".to_string(),
                    arguments: HashMap::new(),
                },
            },
        })
        .await;

    let resp_value = serde_json::to_value(&resp).unwrap();
    assert_eq!(resp_value["jsonrpc"], "2.0");
    assert_eq!(resp_value["id"], 4);
    assert!(resp_value["error"]["code"].as_i64().is_some());
    assert!(resp_value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("missing required argument: name"));
}

#[tokio::test]
async fn prompts_get_unknown_prompt() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(5)),
            body: Requests::PromptsGet {
                params: PromptsGetRequest {
                    name: "unknown_prompt".to_string(),
                    arguments: HashMap::new(),
                },
            },
        })
        .await;

    let resp_value = serde_json::to_value(&resp).unwrap();
    assert_eq!(resp_value["jsonrpc"], "2.0");
    assert_eq!(resp_value["id"], 5);
    assert!(resp_value["error"]["code"].as_i64().is_some());
    assert!(resp_value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("prompt not found"));
}

#[tokio::test]
async fn prompts_get_no_arguments_needed() {
    let prompts = TestPrompts::new();
    let mut server = McpServer::new().prompts(prompts);

    let resp = server
        .handle_request(Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id: Some(RequestId::Int(6)),
            body: Requests::PromptsGet {
                params: PromptsGetRequest {
                    name: "help".to_string(),
                    arguments: HashMap::new(),
                },
            },
        })
        .await;

    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "result": {
                "description": "",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "How can I help you today?"
                        }
                    }
                ]
            }
        })
    );
}
