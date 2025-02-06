use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{get_model, handle_response_openai_compat};
use super::tool_parser::ToolParserProvider;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::{role::Role, tool::Tool, content::TextContent};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;
use regex::Regex;
use chrono::Utc;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we suggest the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
    #[serde(skip)]
    tool_parser: ToolParserProvider,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OllamaProvider::metadata().default_model.to_string());
        OllamaProvider::from_env(model).expect("Failed to initialize Ollama provider")
    }
}

impl OllamaProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model,
            tool_parser: ToolParserProvider::default(),
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        // TODO: remove this later when the UI handles provider config refresh
        // OLLAMA_HOST is sometimes just the 'host' or 'host:port' without a scheme
        let base = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            self.host.clone()
        } else {
            format!("http://{}", self.host)
        };

        let mut base_url = Url::parse(&base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        let explicit_default_port = self.host.ends_with(":80") || self.host.ends_with(":443");
        if base_url.port().is_none() && !explicit_default_port {
            base_url.set_port(Some(OLLAMA_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }

        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

fn create_request_with_tools(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, anyhow::Error> {
    let mut modified_system = system.to_string();
    if !tools.is_empty() {
        // For providers without native tool calling, embed the list of tools directly into the system prompt.
        modified_system.push_str("\nAvailable tools: ");
        let tools_text = serde_json::to_string_pretty(&tools)
            .unwrap_or_else(|_| "[Error serializing tools]".to_string());
        modified_system.push_str(&tools_text);
        modified_system.push_str("\nWhen you want to use a tool, respond with a JSON object in this format: { \"tool\": \"tool_name\", \"args\": { \"arg1\": \"value1\", ... } }");
    }

    create_request(
        model_config,
        &modified_system,
        messages,
        tools,
        &super::utils::ImageFormat::OpenAi,
    )
}

async fn process_tool_calls(message: Message, tool_parser: &ToolParserProvider) -> Message {
    let mut processed = Message {
        role: Role::Assistant,
        created: Utc::now().timestamp(),
        content: vec![],
    };

    // Extract tool calls from the message content
    let text = message.as_concat_text();
    if !text.is_empty() {
        let re = Regex::new(r"\{[^{}]*\}").unwrap(); // Basic regex to find JSON-like structures
        let mut found_valid_json = false;

        for cap in re.find_iter(&text) {
            if let Ok(json) = serde_json::from_str::<Value>(cap.as_str()) {
                if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
                    if let (Some(_tool_name), Some(_args_obj)) = (tool.as_str(), args.as_object()) {
                        found_valid_json = true;
                        processed.content.push(MessageContent::Text(TextContent {
                            text: serde_json::to_string(&json).unwrap(),
                            annotations: None,
                        }));
                    }
                }
            }
        }

        // If no valid JSON was found, try using the tool parser
        if !found_valid_json {
            if let Ok(tool_calls) = tool_parser.parse_tool_calls(&text).await {
                for tool_call in tool_calls {
                    processed.content.push(MessageContent::Text(TextContent {
                        text: serde_json::to_string(&tool_call).unwrap(),
                        annotations: None,
                    }));
                }
            } else {
                // If tool parser fails, pass through the original text
                processed.content.push(MessageContent::Text(TextContent {
                    text: text,
                    annotations: None,
                }));
            }
        }
    }

    processed
}

#[async_trait]
impl Provider for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "ollama",
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            OLLAMA_DOC_URL,
            vec![ConfigKey::new(
                "OLLAMA_HOST",
                true,
                false,
                Some(OLLAMA_HOST),
            )],
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
        let payload = create_request_with_tools(&self.model, system, messages, tools)?;
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let message = process_tool_calls(message, &self.tool_parser).await;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::warn!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
