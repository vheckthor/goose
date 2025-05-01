use anyhow::Result;
use mcp_core::{
    Content, Tool, ToolError,
    protocol::ServerCapabilities,
    resource::Resource,
    prompt::Prompt,
    handler::ResourceError,
};
use mcp_server::router::{Router, RouterService, CapabilitiesBuilder};
use serde_json::{json, Value};

pub struct ToolRouterRouter {
    tools: Vec<Tool>,
}

impl Clone for ToolRouterRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}

impl ToolRouterRouter {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Tool::new(
                    "search_tools",
                    "Search for tools matching a topic",
                    json!({
                        "type": "object",
                        "properties": {
                            "topic": {
                                "type": "string",
                                "description": "Topic to search for tools"
                            }
                        },
                        "required": ["topic"]
                    }),
                    None,
                ),
                Tool::new(
                    "get_tool_schema",
                    "Get full schema for a specific tool",
                    json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name of the tool to get schema for"
                            }
                        },
                        "required": ["name"]
                    }),
                    None,
                ),
            ],
        }
    }

    async fn search_tools(&self, _topic: &str) -> Result<Vec<Content>, ToolError> {
        // In a real implementation, this would search for tools
        // For now, just return a mock response
        let mock_results = json!([
            {
                "name": "developer__shell",
                "description": "Execute a command in the shell"
            },
            {
                "name": "developer__text_editor",
                "description": "Edit text files"
            }
        ]);

        let json_str = match serde_json::to_string(&mock_results) {
            Ok(s) => s,
            Err(e) => return Err(ToolError::ExecutionError(format!("JSON serialization error: {}", e))),
        };
        
        Ok(vec![Content::text(json_str)])
    }

    async fn get_tool_schema(&self, name: &str) -> Result<Vec<Content>, ToolError> {
        // In a real implementation, this would get the schema for a specific tool
        // For now, just return a mock response
        let mock_schema = json!({
            "tool": {
                "name": name,
                "description": "Mock tool description",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "param": {
                            "type": "string"
                        }
                    }
                }
            }
        });

        let json_str = match serde_json::to_string(&mock_schema) {
            Ok(s) => s,
            Err(e) => return Err(ToolError::ExecutionError(format!("JSON serialization error: {}", e))),
        };
        
        Ok(vec![Content::text(json_str)])
    }
}

impl Router for ToolRouterRouter {
    fn name(&self) -> String {
        "toolrouter".to_string()
    }

    fn instructions(&self) -> String {
        "ToolRouter provides dynamic tool discovery and activation at runtime.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let tool_name = tool_name.to_string();
        let this = self.clone();
        
        Box::pin(async move {
            match tool_name.as_str() {
                "search_tools" => {
                    let topic = arguments
                        .get("topic")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::InvalidParameters("Missing 'topic' parameter".to_string()))?;
                    
                    this.search_tools(topic).await
                }
                "get_tool_schema" => {
                    let name = arguments
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::InvalidParameters("Missing 'name' parameter".to_string()))?;
                    
                    this.get_tool_schema(name).await
                }
                _ => Err(ToolError::NotFound(format!("Unknown tool: {}", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async {
            Err(ResourceError::NotFound("Resources not supported".to_string()))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, mcp_core::handler::PromptError>> + Send + 'static>>
    {
        Box::pin(async {
            Err(mcp_core::handler::PromptError::NotFound(
                "Prompts not supported".to_string(),
            ))
        })
    }
}

pub fn create_server() -> RouterService<ToolRouterRouter> {
    RouterService(ToolRouterRouter::new())
}