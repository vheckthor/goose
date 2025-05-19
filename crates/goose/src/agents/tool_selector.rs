use async_trait::async_trait;
use crate::message::Message;
use mcp_core::{Tool, ToolError, Content};

pub enum ToolSelectorStrategy {
    Default,
    Vector,
}

#[derive(Debug, Clone)]
pub struct ToolSelectorContext {
    pub tools: Vec<Tool>,
    pub messages: Vec<Message>,
}

impl ToolSelectorContext {
    pub fn new(tools: Vec<Tool>, messages: Vec<Message>) -> Self {
        Self { tools, messages }
    }
}

#[async_trait]
pub trait ToolSelector: Send + Sync {
    async fn select_tools(&self, ctx: &ToolSelectorContext) -> Result<Vec<Content>, ToolError>;
}

pub struct DefaultToolSelector;

#[async_trait]
impl ToolSelector for DefaultToolSelector {
    async fn select_tools(&self, ctx: &ToolSelectorContext) -> Result<Vec<Content>, ToolError> {
        Ok(ctx.tools.iter()
            .map(|tool| Content::text(tool.name.clone()))
            .collect())
    }
}

pub struct VectorToolSelector;

#[async_trait]
impl ToolSelector for VectorToolSelector {
    async fn select_tools(&self, ctx: &ToolSelectorContext) -> Result<Vec<Content>, ToolError> {
        let mut selected_tools = Vec::new();
        
        if let Some(last_message) = ctx.messages.last() {
            if let Some(content) = last_message.content.first().and_then(|c| c.as_text()) {
                for tool in &ctx.tools {
                    if content.contains(&tool.name) {
                        selected_tools.push(tool.name.clone());
                    }
                }
            }
        }
        
        Ok(selected_tools.into_iter()
            .map(Content::text)
            .collect())
    }
}

// Helper function to create a boxed tool selector
pub fn create_tool_selector(strategy: Option<ToolSelectorStrategy>) -> Box<dyn ToolSelector> {
    match strategy {
        Some(ToolSelectorStrategy::Vector) => Box::new(VectorToolSelector),
        _ => Box::new(DefaultToolSelector),
    }
}
