use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use anyhow::Result;
use serde_json::Value;
use chrono::Utc;
use mcp_core::{role::Role, content::TextContent};
use reqwest::Client;
use std::time::Duration;
use url::Url;

use super::errors::ProviderError;
use super::utils::handle_response_openai_compat;
use super::formats::openai::{create_request, response_to_message};

/// A lightweight provider specifically for parsing tool calls
#[derive(serde::Serialize)]
pub struct ToolParserProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for ToolParserProvider {
    fn default() -> Self {
        let model = ModelConfig::new("mistral".to_string());
        Self::new(model).expect("Failed to initialize tool parser provider")
    }
}

impl ToolParserProvider {
    pub fn new(model: ModelConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host: "http://localhost:11434".to_string(),
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;
        handle_response_openai_compat(response).await
    }

    pub async fn parse_tool_calls(&self, content: &str) -> Result<Vec<Value>> {
        let system = "You are a tool call parser. Your job is to analyze the given text and extract any intended tool calls, formatting them as JSON objects with 'tool' and 'args' fields. Each tool call should be in the format: { \"tool\": \"tool_name\", \"args\": { \"arg1\": \"value1\", ... } }";
        
        let message = Message {
            role: Role::User,
            created: Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: content.to_string(),
                annotations: None,
            })],
        };

        let payload = create_request(
            &self.model,
            system,
            &[message],
            &[],
            &super::utils::ImageFormat::OpenAi,
        )?;

        let response = self.post(payload).await?;
        let message = response_to_message(response)?;
        
        if !message.content.is_empty() {
            if let Some(text) = message.content[0].as_text() {
                if let Ok(json) = serde_json::from_str::<Value>(text) {
                    if let Some(array) = json.as_array() {
                        return Ok(array.to_vec());
                    }
                    // If it's not an array but valid JSON, wrap it in an array
                    return Ok(vec![json]);
                }
            }
        }
        
        Ok(vec![])
    }
}
