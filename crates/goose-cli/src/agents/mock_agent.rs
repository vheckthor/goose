use std::vec;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use goose::{message::Message, providers::base::ProviderUsage, systems::System};
use mcp_client::client::McpClient;

use crate::agents::agent::Agent;

pub struct MockAgent;

#[async_trait]
impl Agent for MockAgent {
    fn add_system(&mut self, _system: Box<dyn System>) {}
    async fn add_mcp_sse_client(&mut self, _uri: String) {}
    async fn add_mcp_stdio_client(&mut self, _cmd: String, _args: Vec<String>) {}

    async fn reply(&mut self, _messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn usage(&self) -> Result<Vec<ProviderUsage>> {
        Ok(vec![ProviderUsage::new(
            "mock".to_string(),
            Default::default(),
            None,
        )])
    }
}
