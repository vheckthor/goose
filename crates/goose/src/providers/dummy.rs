use async_trait::async_trait;
use anyhow::Result;

use super::base::{Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub struct DummyProvider {
    model_config: ModelConfig,
}

impl DummyProvider {
    pub fn new() -> Self {
        Self {
            model_config: ModelConfig::new("dummy".to_string()),
        }
    }
}

#[async_trait]
impl Provider for DummyProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "dummy",
            "Dummy Provider",
            "A placeholder provider that does not process any requests",
            "dummy",
            vec!["dummy".to_string()],
            "",
            vec![],
        )
    }

    async fn complete(
        &self,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        Err(ProviderError::NoProviderConfigured)
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }
}
