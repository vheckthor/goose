use crate::message::Message;
use async_trait::async_trait;
use mcp_core::{Content, ToolError};

pub enum RouterToolSelectionStrategy {
    Default,
    Vector,
}

#[derive(Debug, Clone)]
pub struct RouterToolSelectorContext {
    pub messages: Vec<Message>,
}

impl RouterToolSelectorContext {
    pub fn new(messages: Vec<Message>) -> Self {
        Self { messages }
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
        _ctx: &RouterToolSelectorContext,
    ) -> Result<Vec<Content>, ToolError> {
        Ok(Vec::new())
    }
}

pub struct VectorToolSelector;

#[async_trait]
impl RouterToolSelector for VectorToolSelector {
    async fn select_tools(
        &self,
        _ctx: &RouterToolSelectorContext,
    ) -> Result<Vec<Content>, ToolError> {
        let mut selected_tools = Vec::new();
        // TODO: placeholder for vector tool selection
        Ok(selected_tools)
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
