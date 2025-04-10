use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;

use regex::Regex;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument};

use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::types::ToolResultReceiver;
use crate::config::{Config, ExtensionConfigManager};
use crate::message::Message;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe};
use crate::token_counter::TokenCounter;

use crate::agents::prompt_manager::PromptManager;
use crate::agents::types::SessionConfig;
use mcp_core::{prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolResult};

use super::types::FrontendTool;

/// The main goose Agent
pub struct Agent {
    pub(super) provider: Arc<dyn Provider>,
    pub(super) extension_manager: Mutex<ExtensionManager>,
    pub(super) frontend_tools: HashMap<String, FrontendTool>,
    pub(super) frontend_instructions: Option<String>,
    pub(super) prompt_manager: PromptManager,
    pub(super) token_counter: TokenCounter,
    pub(super) confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(super) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    pub(super) tool_result_rx: ToolResultReceiver,
}

impl Agent {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        Self {
            provider,
            extension_manager: Mutex::new(ExtensionManager::new()),
            frontend_tools: HashMap::new(),
            frontend_instructions: None,
            prompt_manager: PromptManager::new(),
            token_counter,
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
        }
    }

    /// Get a reference count clone to the provider
    pub fn provider(&self) -> Arc<dyn Provider> {
        Arc::clone(&self.provider)
    }

    /// Check if a tool is a frontend tool
    pub fn is_frontend_tool(&self, name: &str) -> bool {
        self.frontend_tools.contains_key(name)
    }

    /// Get a reference to a frontend tool
    pub fn get_frontend_tool(&self, name: &str) -> Option<&FrontendTool> {
        self.frontend_tools.get(name)
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&mut self) -> ExtensionResult<Vec<Tool>> {
        let mut tools = self
            .extension_manager
            .lock()
            .await
            .get_prefixed_tools()
            .await?;

        // Add frontend tools directly - they don't need prefixing since they're already uniquely named
        for frontend_tool in self.frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        Ok(tools)
    }

    #[instrument(skip(self, messages, session), fields(user_message))]
    pub async fn reply(
        &self,
        messages: &[Message],
        session: Option<SessionConfig>,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let mut messages = messages.to_vec();
        let reply_span = tracing::Span::current();
        let mut extension_manager = self.extension_manager.lock().await;
        let mut truncation_attempt: usize = 0;

        // Load settings from config
        let config = Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

        // Setup tools and prompt
        let (mut tools, toolshim_tools, mut system_prompt) = self
            .prepare_tools_and_prompt(&mut extension_manager)
            .await?;

        let (tools_with_readonly_annotation, tools_without_annotation) =
            Self::categorize_tools_by_annotation(&tools);

        // Set the user_message field in the span instead of creating a new event
        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        Ok(Box::pin(async_stream::try_stream! {
            let _reply_guard = reply_span.enter();
            loop {
                match self.generate_response_from_provider(
                    &system_prompt,
                    &messages,
                    &tools,
                    &toolshim_tools,
                ).await {
                    Ok((response, usage)) => {
                        // Update session metrics
                        if let Some(session_config) = session.clone() {
                            Self::update_session_metrics(session_config, &usage, messages.len()).await?;
                        }

                        // Reset truncation attempt
                        truncation_attempt = 0;

                        // Categorize tool requests
                        let (frontend_requests, enable_extension_requests, search_extension_requests, other_requests, filtered_response) =
                            self.categorize_tool_requests(&response);

                        // Yield the assistant's response with frontend tool requests filtered out
                        yield filtered_response.clone();

                        tokio::task::yield_now().await;

                        let all_tool_requests = frontend_requests.len() + enable_extension_requests.len() +
                                              search_extension_requests.len() + other_requests.len();

                        if all_tool_requests == 0 {
                            break;
                        }

                        // Process tool requests depending on goose_mode
                        let mut message_tool_response = Message::user();

                        // First handle any frontend tool requests
                        for request in &frontend_requests {
                            if let Ok(tool_call) = request.tool_call.clone() {
                                // Send frontend tool request and wait for response
                                yield Message::assistant().with_frontend_tool_request(
                                    request.id.clone(),
                                    Ok(tool_call.clone())
                                );

                                if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                                    message_tool_response = message_tool_response.with_tool_response(id, result);
                                }
                            }
                        }

                        // Handle enable extension requests if any
                        let mut extensions_enabled = false;
                        if !enable_extension_requests.is_empty() {
                            let (extension_responses, enabled) = self.handle_enable_extension_requests(
                                &enable_extension_requests,
                                &mut extension_manager
                            ).await;

                            // Merge the tool responses
                            for content in extension_responses.content {
                                message_tool_response.content.push(content);
                            }

                            extensions_enabled = enabled;
                        }

                        // Handle search extension requests
                        if !search_extension_requests.is_empty() {
                            let search_responses = self.handle_search_extension_requests(
                                &search_extension_requests,
                                &extension_manager
                            ).await;

                            // Merge the tool responses
                            for content in search_responses.content {
                                message_tool_response.content.push(content);
                            }
                        }

                        // Handle other tool requests based on goose_mode
                        let other_responses = self.handle_regular_tool_requests(
                            &other_requests,
                            &goose_mode,
                            tools_with_readonly_annotation.clone(),
                            tools_without_annotation.clone(),
                            &extension_manager
                        ).await;

                        // Merge the tool responses
                        for content in other_responses.content {
                            message_tool_response.content.push(content);
                        }

                        // Update system prompt and tools if extensions were enabled
                        if extensions_enabled {
                            let (new_system_prompt, new_tools) = self.update_system_prompt_and_tools_after_install(
                                &mut extension_manager
                            ).await?;

                            system_prompt = new_system_prompt;
                            tools = new_tools;
                        }

                        yield message_tool_response.clone();

                        messages.push(response);
                        messages.push(message_tool_response);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        // Handle context truncation
                        if let Some(error_message) = self.handle_context_truncation(
                            &mut messages,
                            &mut truncation_attempt,
                            &system_prompt,
                            &mut tools
                        ).await {
                            yield error_message;
                            break;
                        }

                        // // Re-acquire the lock if it was dropped during truncation
                        // if extension_manager.is_poisoned() {
                        //     extension_manager = self.extension_manager.lock().await;
                        // }

                        // Retry the loop after truncation
                        continue;
                    },
                    Err(e) => {
                        // Create an error message & terminate the stream
                        error!("Error: {}", e);
                        yield Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error."));
                        break;
                    }
                }

                // Yield control back to the scheduler to prevent blocking
                tokio::task::yield_now().await;
            }
        }))
    }

    /// Extend the system prompt with one line of additional instruction
    pub async fn extend_system_prompt(&mut self, instruction: String) {
        self.prompt_manager.add_system_prompt_extra(instruction);
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&mut self, template: String) {
        self.prompt_manager.set_system_prompt_override(template);
    }

    pub async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                name: _,
                tools,
                instructions,
                bundled: _,
            } => {
                // For frontend tools, just store them in the frontend_tools map
                for tool in tools {
                    let frontend_tool = FrontendTool {
                        name: tool.name.clone(),
                        tool: tool.clone(),
                    };
                    self.frontend_tools.insert(tool.name.clone(), frontend_tool);
                }
                // Store instructions if provided, using "frontend" as the key
                if let Some(instructions) = instructions {
                    self.frontend_instructions = Some(instructions.clone());
                } else {
                    // Default frontend instructions if none provided
                    self.frontend_instructions = Some(
                        "The following tools are provided directly by the frontend and will be executed by the frontend when called.".to_string(),
                    );
                }
            }
            _ => {
                let mut extension_manager = self.extension_manager.lock().await;
                let _ = extension_manager.add_extension(extension).await;
            }
        };

        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .get_prefixed_tools()
            .await
            .unwrap_or_default()
    }

    pub async fn remove_extension(&mut self, name: &str) {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    /// Handle a confirmation response for a tool request
    pub async fn handle_confirmation(
        &self,
        request_id: String,
        confirmation: PermissionConfirmation,
    ) {
        if let Err(e) = self.confirmation_tx.send((request_id, confirmation)).await {
            error!("Failed to send confirmation: {}", e);
        }
    }

    pub async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_prompts()
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        let extension_manager = self.extension_manager.lock().await;

        // First find which extension has this prompt
        let prompts = extension_manager
            .list_prompts()
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return extension_manager
                .get_prompt(extension, name, arguments)
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self) -> anyhow::Result<String> {
        let mut extension_manager = self.extension_manager.lock().await;
        let tools = extension_manager.get_prefixed_tools().await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| {
                ToolInfo::new(
                    &tool.name,
                    &tool.description,
                    get_parameter_names(&tool),
                    None,
                )
            })
            .collect();

        let plan_prompt = extension_manager.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            tracing::error!("Failed to send tool result: {}", e);
        }
    }

    pub async fn create_recipe(&self, mut messages: Vec<Message>) -> Result<Recipe> {
        let mut extension_manager = self.extension_manager.lock().await;
        let extensions_info = extension_manager.get_extensions_info().await;
        let system_prompt = self
            .prompt_manager
            .build_system_prompt(extensions_info, self.frontend_instructions.clone());

        let recipe_prompt = self.prompt_manager.get_recipe_prompt().await;
        let tools = extension_manager.get_prefixed_tools().await?;

        messages.push(Message::user().with_text(recipe_prompt));

        let (result, _usage) = self
            .provider
            .complete(&system_prompt, &messages, &tools)
            .await?;

        let content = result.as_concat_text();

        // the response may be contained in ```json ```, strip that before parsing json
        let re = Regex::new(r"(?s)^```[^\n]*\n(.*?)\n```$").unwrap();
        let clean_content = re
            .captures(&content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
            .unwrap_or(&content)
            .trim()
            .to_string();

        // try to parse json response from the LLM
        let (instructions, activities) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let instructions = json_content
                    .get("instructions")
                    .ok_or_else(|| anyhow!("Missing 'instructions' in json response"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("instructions' is not a string"))?
                    .to_string();

                let activities = json_content
                    .get("activities")
                    .ok_or_else(|| anyhow!("Missing 'activities' in json response"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("'activities' is not an array'"))?
                    .iter()
                    .map(|act| {
                        act.as_str()
                            .map(|s| s.to_string())
                            .ok_or(anyhow!("'activities' array element is not a string"))
                    })
                    .collect::<Result<_, _>>()?;

                (instructions, activities)
            } else {
                // If we can't get valid JSON, try string parsing
                // Use split_once to get the content after "Instructions:".
                let after_instructions = content
                    .split_once("instructions:")
                    .map(|(_, rest)| rest)
                    .unwrap_or(&content);

                // Split once more to separate instructions from activities.
                let (instructions_part, activities_text) = after_instructions
                    .split_once("activities:")
                    .unwrap_or((after_instructions, ""));

                let instructions = instructions_part
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '#')
                    .trim()
                    .to_string();
                let activities_text = activities_text.trim();

                // Regex to remove bullet markers or numbers with an optional dot.
                let bullet_re = Regex::new(r"^[•\-\*\d]+\.?\s*").expect("Invalid regex");

                // Process each line in the activities section.
                let activities: Vec<String> = activities_text
                    .lines()
                    .map(|line| bullet_re.replace(line, "").to_string())
                    .map(|s| s.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                (instructions, activities)
            };

        let extensions = ExtensionConfigManager::get_all().unwrap_or_default();
        let extension_configs: Vec<_> = extensions
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.config.clone())
            .collect();

        let author = Author {
            contact: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            metadata: None,
        };

        let recipe = Recipe::builder()
            .title("Custom recipe from chat")
            .description("a custom recipe instance from this chat session")
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .author(author)
            .build()
            .expect("valid recipe");

        Ok(recipe)
    }
}
