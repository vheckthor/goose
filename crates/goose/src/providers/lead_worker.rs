use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::base::{Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

/// A provider that switches between a lead model and a worker model based on turn count
pub struct LeadWorkerProvider {
    lead_provider: Arc<dyn Provider>,
    worker_provider: Arc<dyn Provider>,
    lead_turns: usize,
    turn_count: Arc<Mutex<usize>>,
}

impl LeadWorkerProvider {
    /// Create a new LeadWorkerProvider
    ///
    /// # Arguments
    /// * `lead_provider` - The provider to use for the initial turns
    /// * `worker_provider` - The provider to use after lead_turns
    /// * `lead_turns` - Number of turns to use the lead provider (default: 3)
    pub fn new(
        lead_provider: Arc<dyn Provider>,
        worker_provider: Arc<dyn Provider>,
        lead_turns: Option<usize>,
    ) -> Self {
        Self {
            lead_provider,
            worker_provider,
            lead_turns: lead_turns.unwrap_or(3),
            turn_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Reset the turn counter (useful for new conversations)
    pub async fn reset_turn_count(&self) {
        let mut count = self.turn_count.lock().await;
        *count = 0;
    }

    /// Get the current turn count
    pub async fn get_turn_count(&self) -> usize {
        *self.turn_count.lock().await
    }

    /// Get the currently active provider based on turn count
    async fn get_active_provider(&self) -> Arc<dyn Provider> {
        let count = *self.turn_count.lock().await;
        if count < self.lead_turns {
            Arc::clone(&self.lead_provider)
        } else {
            Arc::clone(&self.worker_provider)
        }
    }
}

#[async_trait]
impl Provider for LeadWorkerProvider {
    fn metadata() -> ProviderMetadata {
        // This is a wrapper provider, so we return minimal metadata
        ProviderMetadata::new(
            "lead_worker",
            "Lead/Worker Provider",
            "A provider that switches between lead and worker models based on turn count",
            "",     // No default model as this is determined by the wrapped providers
            vec![], // No known models as this depends on wrapped providers
            "",     // No doc link
            vec![], // No config keys as configuration is done through wrapped providers
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the lead provider's model config as the default
        // In practice, this might need to be more sophisticated
        self.lead_provider.get_model_config()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Get the active provider
        let provider = self.get_active_provider().await;

        // Log which provider is being used
        let turn_count = *self.turn_count.lock().await;
        let provider_type = if turn_count < self.lead_turns {
            "lead"
        } else {
            "worker"
        };
        tracing::info!(
            "Using {} provider for turn {} (lead_turns: {})",
            provider_type,
            turn_count + 1,
            self.lead_turns
        );

        // Make the completion request
        let result = provider.complete(system, messages, tools).await;

        // Increment turn count on successful completion
        if result.is_ok() {
            let mut count = self.turn_count.lock().await;
            *count += 1;
        }

        result
    }

    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Combine models from both providers
        let lead_models = self.lead_provider.fetch_supported_models_async().await?;
        let worker_models = self.worker_provider.fetch_supported_models_async().await?;

        match (lead_models, worker_models) {
            (Some(lead), Some(worker)) => {
                let mut all_models = lead;
                all_models.extend(worker);
                all_models.sort();
                all_models.dedup();
                Ok(Some(all_models))
            }
            (Some(models), None) | (None, Some(models)) => Ok(Some(models)),
            (None, None) => Ok(None),
        }
    }

    fn supports_embeddings(&self) -> bool {
        // Support embeddings if either provider supports them
        self.lead_provider.supports_embeddings() || self.worker_provider.supports_embeddings()
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        // Use the lead provider for embeddings if it supports them, otherwise use worker
        if self.lead_provider.supports_embeddings() {
            self.lead_provider.create_embeddings(texts).await
        } else if self.worker_provider.supports_embeddings() {
            self.worker_provider.create_embeddings(texts).await
        } else {
            Err(ProviderError::ExecutionError(
                "Neither lead nor worker provider supports embeddings".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::base::{ProviderMetadata, ProviderUsage, Usage};
    use chrono::Utc;
    use mcp_core::{content::TextContent, Role};

    #[derive(Clone)]
    struct MockProvider {
        name: String,
        model_config: ModelConfig,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message {
                    role: Role::Assistant,
                    created: Utc::now().timestamp(),
                    content: vec![MessageContent::Text(TextContent {
                        text: format!("Response from {}", self.name),
                        annotations: None,
                    })],
                },
                ProviderUsage::new(self.name.clone(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn test_lead_worker_switching() {
        let lead_provider = Arc::new(MockProvider {
            name: "lead".to_string(),
            model_config: ModelConfig::new("lead-model".to_string()),
        });

        let worker_provider = Arc::new(MockProvider {
            name: "worker".to_string(),
            model_config: ModelConfig::new("worker-model".to_string()),
        });

        let provider = LeadWorkerProvider::new(lead_provider, worker_provider, Some(3));

        // First three turns should use lead provider
        for i in 0..3 {
            let (message, usage) = provider.complete("system", &[], &[]).await.unwrap();
            assert_eq!(usage.model, "lead");
            assert_eq!(provider.get_turn_count().await, i + 1);
        }

        // Subsequent turns should use worker provider
        for i in 3..6 {
            let (message, usage) = provider.complete("system", &[], &[]).await.unwrap();
            assert_eq!(usage.model, "worker");
            assert_eq!(provider.get_turn_count().await, i + 1);
        }

        // Reset and verify it goes back to lead
        provider.reset_turn_count().await;
        assert_eq!(provider.get_turn_count().await, 0);

        let (message, usage) = provider.complete("system", &[], &[]).await.unwrap();
        assert_eq!(usage.model, "lead");
    }
}
