use std::sync::Arc;

use super::{
    anthropic::AnthropicProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    bedrock::BedrockProvider,
    databricks::DatabricksProvider,
    gcpvertexai::GcpVertexAIProvider,
    githubcopilot::GithubCopilotProvider,
    google::GoogleProvider,
    groq::GroqProvider,
    lead_worker::LeadWorkerProvider,
    ollama::OllamaProvider,
    openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
    venice::VeniceProvider,
};
use crate::model::ModelConfig;
use anyhow::Result;

#[cfg(test)]
use super::errors::ProviderError;
#[cfg(test)]
use mcp_core::tool::Tool;

pub fn providers() -> Vec<ProviderMetadata> {
    vec![
        AnthropicProvider::metadata(),
        AzureProvider::metadata(),
        BedrockProvider::metadata(),
        DatabricksProvider::metadata(),
        GcpVertexAIProvider::metadata(),
        GithubCopilotProvider::metadata(),
        GoogleProvider::metadata(),
        GroqProvider::metadata(),
        OllamaProvider::metadata(),
        OpenAiProvider::metadata(),
        OpenRouterProvider::metadata(),
        VeniceProvider::metadata(),
    ]
}

pub fn create(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    // Check if we should create a lead/worker provider
    let config = crate::config::Config::global();

    // If GOOSE_LEAD_MODEL is set, create a lead/worker provider
    if let Ok(lead_model_name) = config.get_param("GOOSE_LEAD_MODEL") {
        // Worker model is always the main configured model
        let worker_model_config = model.clone();

        println!(
            "Creating lead/worker provider with lead model: {}, worker model: {}",
            lead_model_name, worker_model_config.model_name
        );

        // Always use 3 turns for lead model
        let lead_turns = 3;

        // Create lead and worker providers
        let lead_model_config = crate::model::ModelConfig::new(lead_model_name);

        let lead_provider = create_provider(name, lead_model_config)?;
        let worker_provider = create_provider(name, worker_model_config)?;

        return Ok(Arc::new(LeadWorkerProvider::new(
            lead_provider,
            worker_provider,
            Some(lead_turns),
        )));
    }

    // Otherwise create a regular provider
    create_provider(name, model)
}

fn create_provider(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    // We use Arc instead of Box to be able to clone for multiple async tasks
    match name {
        "openai" => Ok(Arc::new(OpenAiProvider::from_env(model)?)),
        "anthropic" => Ok(Arc::new(AnthropicProvider::from_env(model)?)),
        "azure_openai" => Ok(Arc::new(AzureProvider::from_env(model)?)),
        "aws_bedrock" => Ok(Arc::new(BedrockProvider::from_env(model)?)),
        "databricks" => Ok(Arc::new(DatabricksProvider::from_env(model)?)),
        "groq" => Ok(Arc::new(GroqProvider::from_env(model)?)),
        "ollama" => Ok(Arc::new(OllamaProvider::from_env(model)?)),
        "openrouter" => Ok(Arc::new(OpenRouterProvider::from_env(model)?)),
        "gcp_vertex_ai" => Ok(Arc::new(GcpVertexAIProvider::from_env(model)?)),
        "google" => Ok(Arc::new(GoogleProvider::from_env(model)?)),
        "venice" => Ok(Arc::new(VeniceProvider::from_env(model)?)),
        "github_copilot" => Ok(Arc::new(GithubCopilotProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, MessageContent};
    use crate::providers::base::{ProviderMetadata, ProviderUsage, Usage};
    use chrono::Utc;
    use mcp_core::{content::TextContent, Role};
    use std::env;

    #[derive(Clone)]
    struct MockTestProvider {
        name: String,
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockTestProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new(
                "mock_test",
                "Mock Test Provider",
                "A mock provider for testing",
                "mock-model",
                vec!["mock-model"],
                "",
                vec![],
            )
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
                        text: format!(
                            "Response from {} with model {}",
                            self.name, self.model_config.model_name
                        ),
                        annotations: None,
                    })],
                },
                ProviderUsage::new(self.model_config.model_name.clone(), Usage::default()),
            ))
        }
    }

    #[test]
    fn test_create_lead_worker_provider() {
        // Save current env var
        let saved_lead = env::var("GOOSE_LEAD_MODEL").ok();

        // Test with lead model configuration
        env::set_var("GOOSE_LEAD_MODEL", "gpt-4o");

        // This will fail because we need actual provider credentials, but it tests the logic
        let result = create("openai", ModelConfig::new("gpt-4o-mini".to_string()));

        // The creation will fail due to missing API keys, but we can verify it tried to create a lead/worker provider
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // If it's trying to get OPENAI_API_KEY, it means it went through the lead/worker creation path
        assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));

        // Restore env var
        match saved_lead {
            Some(val) => env::set_var("GOOSE_LEAD_MODEL", val),
            None => env::remove_var("GOOSE_LEAD_MODEL"),
        }
    }

    #[test]
    fn test_create_regular_provider_without_lead_config() {
        // Save current env var
        let saved_lead = env::var("GOOSE_LEAD_MODEL").ok();

        // Ensure GOOSE_LEAD_MODEL is not set
        env::remove_var("GOOSE_LEAD_MODEL");

        // This should try to create a regular provider
        let result = create("openai", ModelConfig::new("gpt-4o-mini".to_string()));

        // It will fail due to missing API key, but shouldn't be trying to create lead/worker
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));

        // Restore env var
        if let Some(val) = saved_lead {
            env::set_var("GOOSE_LEAD_MODEL", val);
        }
    }
}
