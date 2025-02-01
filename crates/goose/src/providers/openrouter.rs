use anyhow::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use mcp_core::{content::TextContent, tool::ToolCall};
use mcp_core::tool::Tool;
use url::Url;

// Helper function to create a message with text content
fn create_text_message(text: String) -> Message {
    let mut msg = Message::assistant();
    msg.content = vec![MessageContent::Text(TextContent { 
        text,
        annotations: None,
    })];
    msg
}

// Helper function to create a message with tool request
fn create_tool_message(command: String) -> Message {
    let mut msg = Message::assistant();
    msg.content = vec![MessageContent::ToolRequest(ToolRequest {
        id: "1".to_string(), // Fixed ID since we only have one tool call
        tool_call: Ok(ToolCall {
            name: "developer__shell".to_string(),
            arguments: serde_json::json!({ "command": command }),
        }),
    })];
    msg
}

pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";
pub const OPENROUTER_MODEL_PREFIX_ANTHROPIC: &str = "anthropic";

// OpenRouter can run many models, we suggest the default
pub const OPENROUTER_KNOWN_MODELS: &[&str] = &[OPENROUTER_DEFAULT_MODEL];
pub const OPENROUTER_DOC_URL: &str = "https://openrouter.ai/models";

#[derive(serde::Serialize)]
pub struct OpenRouterProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for OpenRouterProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenRouterProvider::metadata().default_model);
        OpenRouterProvider::from_env(model).expect("Failed to initialize OpenRouter provider")
    }
}

impl OpenRouterProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENROUTER_API_KEY")?;
        let host: String = config
            .get("OPENROUTER_HOST")
            .unwrap_or_else(|_| "https://openrouter.ai".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("api/v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/block/goose")
            .header("X-Title", "Goose")
            .json(&payload)
            .send()
            .await?;

        handle_response_openai_compat(response).await
    }
}

/// Update the request when using anthropic model.
/// For anthropic model, we can enable prompt caching to save cost. Since openrouter is the OpenAI compatible
/// endpoint, we need to modify the open ai request to have anthropic cache control field.
fn update_request_for_anthropic(original_payload: &Value) -> Value {
    let mut payload = original_payload.clone();

    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        // Add "cache_control" to the last and second-to-last "user" messages.
        // During each turn, we mark the final message with cache_control so the conversation can be
        // incrementally cached. The second-to-last user message is also marked for caching with the
        // cache_control parameter, so that this checkpoint can read from the previous cache.
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        // Update the system message to have cache_control field.
        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }
    payload
}

fn create_request_based_on_model(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    // For deepseek models, we want to include tools in the system prompt instead
    if model_config.model_name.contains("deepseek-r1") {
        let tool_instructions = if !tools.is_empty() {
            let tool_descriptions: Vec<String> = tools.iter()
                .map(|tool| format!("- {}: {}", tool.name, tool.description))
                .collect();
            
            println!("\nTools being provided:\n{}", tool_descriptions.join("\n"));
            
            format!(
                "\n\nAvailable tools:\n{}\n\n# Reminder: Instructions for Tool Use\n\nTool uses are formatted using XML-style tags. The tool name is enclosed in opening and closing tags. Here's the structure:\n\n<tool_name>\n<parameter1_name>value1</parameter1_name>\n<parameter2_name>value2</parameter2_name>\n...\n</tool_name>\n\nFor example, to use the shell tool:\n\n<developer__shell>\n<command>ls -l</command>\n</developer__shell>\n\nAlways adhere to this format for all tool uses to ensure proper parsing and execution.\n",
                tool_descriptions.join("\n")
            )
        } else {
            String::new()
        };
        
        let enhanced_system = format!("{}{}", system, tool_instructions);
        println!("\nEnhanced system prompt:\n{}", enhanced_system);
        
        let mut payload = create_request(
            model_config,
            &enhanced_system,
            messages,
            &[], // Pass empty tools array since we're handling them in the system prompt
            &super::utils::ImageFormat::OpenAi,
        )?;
        return Ok(payload);
    }

    let mut payload = create_request(
        model_config,
        system,
        messages,
        tools,
        &super::utils::ImageFormat::OpenAi,
    )?;

    if model_config
        .model_name
        .starts_with(OPENROUTER_MODEL_PREFIX_ANTHROPIC)
    {
        payload = update_request_for_anthropic(&payload);
    }

    Ok(payload)
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openrouter",
            "OpenRouter",
            "Router for many model providers",
            OPENROUTER_DEFAULT_MODEL,
            OPENROUTER_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPENROUTER_DOC_URL,
            vec![
                ConfigKey::new("OPENROUTER_API_KEY", true, true, None),
                ConfigKey::new(
                    "OPENROUTER_HOST",
                    false,
                    false,
                    Some("https://openrouter.ai"),
                ),
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
        // Create the base payload
        let payload = create_request_based_on_model(&self.model, system, messages, tools)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response - special handling for deepseek models
        let message = if self.model.model_name.contains("deepseek-r1") {
            // For deepseek models, look for XML-style tool calls
            let content = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            println!("Response payload:\n{}", serde_json::to_string_pretty(&response).unwrap());
            println!("\nExtracted content:\n{}", content);

            // Check for either <shell> or <developer__shell> tags
            if content.contains("<shell>") || content.contains("<developer__shell>") {
                // Extract command from either tag format
                let command = if content.contains("<developer__shell>") {
                    content
                        .split("<command>")
                        .nth(1)
                        .and_then(|s| s.split("</command>").next())
                } else {
                    content
                        .split("<command>")
                        .nth(1)
                        .and_then(|s| s.split("</command>").next())
                };

                if let Some(cmd) = command {
                    println!("\nExtracted command: {}", cmd);
                    create_tool_message(cmd.trim().to_string())
                } else {
                    create_text_message(content)
                }
            } else {
                create_text_message(content)
            }
        } else {
            response_to_message(response.clone())?
        };

        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::warn!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}