use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::{Message, MessageContent};
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

    async fn post(&self, mut payload: Value) -> Result<Value, ProviderError> {
        let base_url = format!("https://router.huggingface.co/{}/v1/chat/completions", self.provider);
        
        // Check if tools are present and add tool_choice if needed
        let mut should_use_shell = false;
        
        // First check if there's a shell tool
        if let Some(tools) = payload.get("tools") {
            if let Some(tools_array) = tools.as_array() {
                for tool in tools_array.iter() {
                    if let Some(function) = tool.get("function") {
                        if let Some(name) = function.get("name") {
                            if name.as_str() == Some("developer__shell") {
                                should_use_shell = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        // Check if we're handling a tool response
        let mut has_tool_response = false;
        if let Some(messages) = payload.get("messages") {
            if let Some(messages_array) = messages.as_array() {
                // Check if any message is a tool response
                for message in messages_array {
                    if message.get("role").and_then(|r| r.as_str()) == Some("tool") {
                        has_tool_response = true;
                        break;
                    }
                }
            }
        }
        
        // Now add the tool_choice
        if let Some(tools) = payload.get("tools") {
            if !tools.as_array().unwrap_or(&vec![]).is_empty() {
                if let Some(obj) = payload.as_object_mut() {
                    if has_tool_response {
                        // After a tool response, use "none" for tool_choice
                        obj.insert("tool_choice".to_string(), json!("none"));
                    } else if should_use_shell {
                        // For initial request with shell tool, direct to use shell
                        obj.insert("tool_choice".to_string(), json!({
                            "type": "function",
                            "function": {
                                "name": "developer__shell"
                            }
                        }));
                    } else {
                        // Default case
                        obj.insert("tool_choice".to_string(), json!("none"));
                    }
                }
            }
        }
        
        // For requests with tool responses, we need to modify the payload to work around the API limitation
        if has_tool_response {
            // Remove tools completely to avoid the concatenation error
            if let Some(obj) = payload.as_object_mut() {
                obj.remove("tools");
                obj.insert("tool_choice".to_string(), json!("none"));
            }
        }
        
        //println!("REQUEST PAYLOAD: {:?}", payload);
        let request = self
            .client
            .post(&base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload);

        let response = request.send().await?;
        //println!("RAW RESPONSE: {:?}", response);
        let result = handle_response_openai_compat(response).await;
        if let Err(ref e) = result {
            println!("ERROR: {:?}", e);
        }
        // else if let Ok(ref value) = result {
        //    println!("RESPONSE BODY: {:?}", value);
        //}
        result
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
        // Check if this is a tool response
        let is_tool_response = messages.last().map_or(false, |m| {
            m.content.iter().any(|c| matches!(c, MessageContent::ToolResponse(_)))
        });

        if is_tool_response {
            // For tool responses, we need to create a simplified payload without tools
            
            // Extract the tool response content
            let tool_output = messages.last()
                .and_then(|m| m.content.iter().find_map(|c| {
                    if let MessageContent::ToolResponse(resp) = c {
                        if let Ok(contents) = &resp.tool_result {
                            // Extract text content from the tool result
                            let text = contents.iter()
                                .filter_map(|content| {
                                    match content {
                                        mcp_core::Content::Text(text) => Some(text.text.clone()),
                                        _ => None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            Some(text)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }))
                .unwrap_or_else(|| "".to_string());
                
            // Create a new set of messages for the model
            // We'll use a completely new conversation to avoid any issues with the API
            let mut new_messages = Vec::new();
            
            // Add a system message with the tool output
            let system_with_output = format!(
                "{}\n\nThe following command was executed and produced this output:\n\n```\n{}\n```\n\nPlease analyze this output and provide a helpful response.",
                system,
                tool_output
            );
            
            // Add a simple user message
            new_messages.push(Message::user().with_text("Please describe what you see in the command output above."));
            
            // Create a request without tools and with the new conversation
            let simple_payload = create_request(&self.model, &system_with_output, &new_messages, &[], &ImageFormat::OpenAi)?;
            
            // Make request with simplified payload
            let response = self.post(simple_payload.clone()).await?;
            
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
            emit_debug_trace(&self.model, &simple_payload, &response, &usage);
            Ok((message, ProviderUsage::new(model, usage)))
        } else {
            // Normal flow for initial request
            let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
            
            // Make request - post method handles tool responses
            let response = self.post(payload.clone()).await?;
            
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
}