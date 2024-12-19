use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use goose::{
    mcp_agent::McpAgent as GooseAgent, message::Message, providers::base::ProviderUsage,
    systems::System,
};

#[async_trait]
pub trait Agent {
    fn add_system(&mut self, system: Box<dyn System>);

    async fn add_mcp_sse_client(&mut self, uri: String);
    async fn add_mcp_stdio_client(&mut self, cmd: String, args: Vec<String>);

    async fn reply(&mut self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>>;
    async fn usage(&self) -> Result<Vec<ProviderUsage>>;
}

#[async_trait]
impl Agent for GooseAgent {
    fn add_system(&mut self, _system: Box<dyn System>) {}

    async fn add_mcp_sse_client(&mut self, uri: String) {
        self.add_mcp_sse_client(uri).await;
    }

    async fn add_mcp_stdio_client(&mut self, cmd: String, args: Vec<String>) {
        self.add_mcp_stdio_client(cmd, args).await;
    }

    async fn reply(&mut self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        self.reply(messages).await
    }

    async fn usage(&self) -> Result<Vec<ProviderUsage>> {
        self.usage().await
    }
}
