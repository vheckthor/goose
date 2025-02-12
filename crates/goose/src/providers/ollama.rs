use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{get_model, handle_response_openai_compat};
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use indoc::formatdoc;
use mcp_core::tool::{Tool, ToolCall};
use uuid;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref THINK_PATTERN: Regex = Regex::new(r"<think>(.*?)</think>").unwrap();
    static ref ANSWER_SCHEMA: Value = json!({
        "type": "object",
        "properties": {
            "problem": {"type": "string"},
            "solution": {"type": "number"},
            "explanation": {"type": "string"}
        },
        "required": ["problem", "solution", "explanation"]
    });
}

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OllamaProvider::metadata().default_model);
        OllamaProvider::from_env(model).expect("Failed to initialize Ollama provider")
    }
}

impl OllamaProvider {
    fn parse_thinking_response(&self, response: &Value) -> Option<String> {
        response.get("message")
            .and_then(|msg| msg.get("content"))
            .and_then(|content| content.as_str())
            .and_then(|content| {
                THINK_PATTERN.captures(content)
                    .map(|cap| cap[1].trim().to_string())
            })
    }

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
        })
    }

    async fn post(&self, mut payload: Value) -> Result<Value, ProviderError> {
        // Get initial response without format parameter
        payload["stream"] = json!(false);
        
        // Make initial request
        let initial_response = self.make_request(payload.clone()).await?;
        
        // Check for thinking tags
        if let Some(thinking_content) = self.parse_thinking_response(&initial_response) {
            // Add thinking content as assistant message
            if let Value::Array(ref mut messages) = payload["messages"] {
                messages.push(json!({
                    "role": "assistant",
                    "content": thinking_content
                }));
                messages.push(json!({
                    "role": "user",
                    "content": "return a json in the format"
                }));
            }
            
            // Add format parameter for structured output
            payload["format"] = ANSWER_SCHEMA.clone();
            
            // Make second request with structured output format
            return self.make_request(payload).await;
        }
        
        // If no thinking tags, handle tool calls format
        if payload.get("tools").is_some() {
            let tool_call_schema = json!({
                "type": "object",
                "properties": {
                    "reply_to_user": {
                        "type": "string"
                    },
                    "tool_calls": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {  // Added missing properties wrapper
                                "name": {
                                    "type": "string"
                                },
                                "arguments": {
                                    "type": "object",
                                    "additionalProperties": true
                                }
                            },
                            "required": ["name", "arguments"]  // Added required fields
                        }
                    }
                },
                "required": ["reply_to_user", "tool_calls"]
            });
            payload["format"] = tool_call_schema;
            return self.make_request(payload).await;
        }
        
        Ok(initial_response)
    }
    
    async fn make_request(&self, payload: Value) -> Result<Value, ProviderError> {
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

        let url = base_url.join("api/chat").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let payload = if let Value::Object(mut map) = payload {
            map.remove("tools");
            Value::Object(map)
        } else {
            payload
        };
        // println!("=====PAYLOAD====:\n{:?}", serde_json::to_string_pretty(&payload));

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }

    fn parse_tool_call_response(&self, response: &Value) -> Result<Message, ProviderError> {
        let mut message = Message::assistant();
        
        if let Some(message_obj) = response.get("message") {
            if let Some(content) = message_obj.get("content").and_then(|c| c.as_str()) {
                // Try to parse the content as JSON
                if let Ok(json) = serde_json::from_str::<Value>(content) {
                    // Extract reply_to_user if present
                    if let Some(reply) = json.get("reply_to_user").and_then(|r| r.as_str()) {
                        message = message.with_text(reply.to_string());
                    }
    
                    // Extract tool calls if present
                    if let Some(tool_calls) = json.get("tool_calls").and_then(|tc| tc.as_array()) {
                        for tool_call in tool_calls {
                            if let (Some(name), Some(arguments)) = (
                                tool_call.get("name").and_then(|n| n.as_str()),
                                tool_call.get("arguments")
                            ) {
                                message = message.with_tool_request(
                                    uuid::Uuid::new_v4().to_string(),
                                    Ok(ToolCall::new(
                                        name.to_string(),
                                        arguments.clone()
                                    )),
                                );
                            }
                        }
                    }
                    
                    return Ok(message);
                }
            }
            
            // Fallback to regular message parsing if JSON parsing fails
            return Ok(response_to_message(response.clone())?);
        }
        
        // If we can't find a message object, try to parse as regular message
        Ok(response_to_message(response.clone())?)
    }

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
        // Transform the system message to replace developer instructions
        let modified_system = if let Some(dev_section) = system.split("## developer").nth(1) {
            if let (Some(start_idx), Some(end_idx)) = (
                dev_section.find("### Instructions"),
                dev_section.find("operating system:"),
            ) {
                let new_instructions = formatdoc! {r#"
        The Developer extension enables you to edit code files, execute shell commands, and capture screen/window content. These tools allow for various development and debugging workflows.
        Available Tools:
        1. Shell Execution
        Tool Name: developer__shell
        Schema:
        {{
          "command": "string"  // The shell command to execute
        }}
        Description: Executes commands in the shell and returns the combined output and error messages.
        Use cases:
        - Running scripts: `python script.py`
        - Installing dependencies: `pip install -r requirements.txt`
        - Checking system information: `uname -a`, `df -h`
        - Searching for files or text: **Use `rg` (ripgrep) instead of `find` or `ls -r`**
          - Find a file: `rg --files | rg example.py`
          - Search within files: `rg 'class Example'`
        Best Practices:
        - **Avoid commands with large output** (pipe them to a file if necessary).
        - **Run background processes** if they take a long time (e.g., `uvicorn main:app &`).
        - **git commands can be run on the shell, however if the git extension is installed, you should use the git tool instead.
        - **If the shell command is a rm, mv, or cp, you should verify with the user before running the command.

        2. Text Editor
        Tool Name: developer__text_editor
        Schema:
        {{
          "command": "string",     // One of: "view", "write", "str_replace", "undo_edit"
          "file_path": "string",   // Absolute path to the file
          "file_text": "string",   // Required for "write" command
          "old_str": "string",     // Required for "str_replace" command
          "new_str": "string"      // Required for "str_replace" command
        }}
        Description: Performs file-based operations such as viewing, writing, replacing text, and undoing edits.
        Commands:
        - view: Read the content of a file.
        - write: Create or overwrite a file. Caution: Overwrites the entire file!
        - str_replace: Replace a specific string in a file.
        - undo_edit: Revert the last edit.
        Example Usage:
        text_editor(command="view", file_path="/absolute/path/to/file.py")
        text_editor(command="write", file_path="/absolute/path/to/file.py", file_text="print('hello world')")
        text_editor(command="str_replace", file_path="/absolute/path/to/file.py", old_str="hello world", new_str="goodbye world")
        text_editor(command="undo_edit", file_path="/absolute/path/to/file.py")
        Protocol for Text Editor:
        For edit and replace commands, please verify what you are editing with the user before running the command.
        - User: "Please edit the file /absolute/path/to/file.py"
        - Assistant: "Ok sounds good, I'll be editing the file /absolute/path/to/file.py and creating modifications xyz to the file. Let me know whether you'd like to proceed."
        - User: "Yes, please proceed."
        - Assistant: "I've created the modifications xyz to the file /absolute/path/to/file.py"

        3. List Windows
        Tool Name: developer__list_windows
        Schema:
        {{}}  // No arguments required
        Description: Lists all visible windows with their titles.
        Use this to find window titles for screen capture.

        4. Screen Capture
        Tool Name: developer__screen_capture
        Schema:
        {{
          "display": "number",       // Optional: Display number to capture (e.g., 0 for main display)
          "window_title": "string"   // Optional: Title of window to capture
        }}
        Description: Takes a screenshot of a display or specific window.
        Options:
        - Capture display: `screen_capture(display=0)`  # Main display
        - Capture window: `screen_capture(window_title="Window Title")`
        Info: at the start of the session, the user's directory is:
        "#};

                let before_dev = system.split("## developer").next().unwrap_or("");
                let after_marker = &dev_section[end_idx..];

                format!(
                    "{}## developer{}### Instructions\n{}{}",
                    before_dev,
                    &dev_section[..start_idx],
                    new_instructions,
                    after_marker
                )
            } else {
                system.to_string()
            }
        } else {
            system.to_string()
        };

        // Transform messages to convert tool calls into regular messages
        let transformed_messages: Vec<Message> = messages.iter().map(|msg| {
            if msg.is_tool_call() {
                // Convert tool call request to assistant message
                let content = msg.content.iter().find_map(|c| {
                    if let MessageContent::ToolRequest(req) = c {
                        Some(serde_json::to_string(&req.tool_call).unwrap_or_default())
                    } else {
                        None
                    }
                }).unwrap_or_default();
                Message::assistant().with_text(content)
            } else if msg.is_tool_response() {
                // Convert tool call response to user message
                let content = msg.content.iter().find_map(|c| {
                    if let MessageContent::ToolResponse(resp) = c {
                        match &resp.tool_result {
                            Ok(contents) => Some(contents.iter()
                                .filter_map(|c| c.as_text())
                                .collect::<Vec<_>>()
                                .join("\n")),
                            Err(e) => Some(e.to_string())
                        }
                    } else {
                        None
                    }
                }).unwrap_or_default();
                Message::user().with_text(content)
            } else {
                msg.clone()
            }
        }).collect();

        let payload = create_request(
            &self.model,
            &modified_system,
            &transformed_messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        let response = self.post(payload.clone()).await?;
        println!("======RESPONSE====\n{:?}", response);

        // Parse response
        let message = self.parse_tool_call_response(&response)?;
        println!("MESSAGE IS {:?}\n", message);
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
