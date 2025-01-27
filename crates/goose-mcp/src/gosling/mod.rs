use indoc::indoc;
use serde_json::json;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, sync::Mutex};

use mcp_core::{
    handler::ToolError,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

/// A simpler extension designed as a starting point for new extensions
#[derive(Clone)]
pub struct GoslingRouter {
    tools: Vec<Tool>,
    active_resources: Arc<Mutex<HashMap<String, Resource>>>,
    instructions: String,
}

impl Default for GoslingRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl GoslingRouter {
    pub fn new() -> Self {
        // Create tools for the system
        let example_tool = Tool::new(
            "example",
            indoc! {r#"
                A simple example tool that echoes back the input message.
            "#},
            json!({
                "type": "object",
                "required": ["message"],
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                }
            }),
        );

        let instructions = indoc! {r#"
            This is a simple starter extension that demonstrates the basic structure.
            It provides a single example tool that echoes back messages.
            "#};

        Self {
            tools: vec![example_tool],
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            instructions: instructions.to_string(),
        }
    }

    // Implement example tool functionality
    async fn example(&self, params: serde_json::Value) -> Result<Vec<Content>, ToolError> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'message' parameter".into()))?;

        Ok(vec![Content::text(format!("Echo: {}", message))])
    }
}

impl Router for GoslingRouter {
    fn name(&self) -> String {
        "GoslingExtension".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "example" => this.example(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        let active_resources = self.active_resources.lock().unwrap();
        active_resources.values().cloned().collect()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, mcp_core::handler::ResourceError>> + Send + 'static>>
    {
        Box::pin(async move {
            Err(mcp_core::handler::ResourceError::NotFound(
                "Resource not found".into(),
            ))
        })
    }
}