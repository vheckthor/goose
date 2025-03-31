use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const HUGGINGFACE_DEFAULT_MODEL: &str = "deepseek-ai/DeepSeek-V3-0324-fast";
pub const HUGGINGFACE_KNOWN_MODELS: &[&str] = &[
    "deepseek-ai/DeepSeek-V3-0324-fast",
    "mistralai/Mistral-7B-Instruct-v0.2",
    "meta-llama/Llama-2-70b-chat-hf",
    "google/gemma-7b",
    "google/gemma-2b",
];

pub const HUGGINGFACE_DOC_URL: &str = "https://huggingface.co/models";

#[derive(Debug, serde::Serialize)]
pub struct HuggingFaceProvider {
    #[serde(skip)]
    client: Client,
    api_key: String,
    provider: String,
    model: ModelConfig,
}

impl Default for HuggingFaceProvider {
    fn default() -> Self {
        let model = ModelConfig::new(HuggingFaceProvider::metadata().default_model);
        HuggingFaceProvider::from_env(model).expect("Failed to initialize HuggingFace provider")
    }
}

impl HuggingFaceProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("HUGGINGFACE_TOKEN")?;
        let provider: String = config.get_param("HUGGINGFACE_PROVIDER").unwrap_or_else(|_| "nebius".to_string());
        let timeout_secs: u64 = config.get_param("HUGGINGFACE_TIMEOUT").unwrap_or(600);
        
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            api_key,
            provider,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = format!("https://router.huggingface.co/{}/v1/chat/completions", self.provider);
        
        let request = self
            .client
            .post(&base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload);

        let response = request.send().await?;

        println!("RAW RESPONSE: {:?}", response);
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for HuggingFaceProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "huggingface",
            "HuggingFace",
            "Access models hosted on HuggingFace through their OpenAI-compatible API",
            HUGGINGFACE_DEFAULT_MODEL,
            HUGGINGFACE_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            HUGGINGFACE_DOC_URL,
            vec![
                ConfigKey::new("HUGGINGFACE_TOKEN", true, true, None),
                ConfigKey::new("HUGGINGFACE_PROVIDER", false, false, Some("nebius")),
                ConfigKey::new("HUGGINGFACE_TIMEOUT", false, false, Some("600")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        println!("Response: {:?}", response);

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}