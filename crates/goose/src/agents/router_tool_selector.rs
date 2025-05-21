use crate::message::Message;
use async_trait::async_trait;
use mcp_core::{Content, Tool, ToolError};

pub enum RouterToolSelectionStrategy {
    Default,
    Vector,
}

#[derive(Debug, Clone)]
pub struct RouterToolSelectorContext {
    pub tools: Vec<Tool>,
    pub messages: Vec<Message>,
}

impl RouterToolSelectorContext {
    pub fn new(tools: Vec<Tool>, messages: Vec<Message>) -> Self {
        Self { tools, messages }
    }
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(
        &self,
        ctx: &RouterToolSelectorContext,
    ) -> Result<Vec<Content>, ToolError>;
}

pub struct DefaultToolSelector;

#[async_trait]
impl RouterToolSelector for DefaultToolSelector {
    async fn select_tools(
        &self,
        ctx: &RouterToolSelectorContext,
    ) -> Result<Vec<Content>, ToolError> {
        Ok(ctx
            .tools
            .iter()
            .map(|tool| Content::text(tool.name.clone()))
            .collect())
    }
}

pub struct VectorToolSelector;

#[async_trait]
impl RouterToolSelector for VectorToolSelector {
    async fn select_tools(
        &self,
        ctx: &RouterToolSelectorContext,
    ) -> Result<Vec<Content>, ToolError> {
        let mut selected_tools = Vec::new();
        // TODO: placeholder for vector tool selection
        if let Some(last_message) = ctx.messages.last() {
            if let Some(content) = last_message.content.first().and_then(|c| c.as_text()) {
                for tool in &ctx.tools {
                    if content.contains(&tool.name) {
                        selected_tools.push(tool.name.clone());
                    }
                }
            }
        }

        Ok(selected_tools.into_iter().map(Content::text).collect())
    }
}

// Helper function to create a boxed tool selector
pub fn create_tool_selector(
    strategy: Option<RouterToolSelectionStrategy>,
) -> Box<dyn RouterToolSelector> {
    match strategy {
        Some(RouterToolSelectionStrategy::Vector) => Box::new(VectorToolSelector),
        _ => Box::new(DefaultToolSelector),
    }
}
