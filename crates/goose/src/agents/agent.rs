use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;

use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument};

use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::types::ToolResultReceiver;
use crate::config::Config;
use crate::message::Message;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use crate::token_counter::TokenCounter;

use mcp_core::{prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolResult};

use crate::agents::platform_tools::{self};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::types::{FrontendTool, SessionConfig};

use crate::agents::agent_context::handle_truncation_error;
use crate::agents::agent_message::{
    create_error_response, process_provider_response, update_session_metrics,
};

/// The main goose Agent
pub struct Agent {
    provider: Arc<dyn Provider>,
    extension_manager: Mutex<ExtensionManager>,
    pub(crate) frontend_tools: HashMap<String, FrontendTool>,
    pub(crate) frontend_instructions: Option<String>,
    pub(crate) prompt_manager: PromptManager,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(crate) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    pub(crate) tool_result_rx: ToolResultReceiver,
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

    /// Get a borrowed reference to the token counter
    pub fn token_counter(&self) -> &TokenCounter {
        &self.token_counter
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

    pub async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                name: _,
                tools,
                instructions,
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

    /// Prepares tools and system prompt for a provider request
    async fn prepare_tools_and_prompt(
        &self,
        extension_manager: &mut ExtensionManager,
    ) -> anyhow::Result<(Vec<Tool>, Vec<Tool>, String)> {
        // Get tools from extension manager
        let mut tools = extension_manager.get_prefixed_tools().await?;

        // Add resource tools if supported
        if extension_manager.supports_resources() {
            tools.push(platform_tools::read_resource_tool());
            tools.push(platform_tools::list_resources_tool());
        }

        // Add platform tools
        tools.push(platform_tools::search_available_extensions_tool());
        tools.push(platform_tools::enable_extension_tool());

        // Add frontend tools
        for frontend_tool in self.frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        // Prepare system prompt
        let extensions_info = extension_manager.get_extensions_info().await;
        let mut system_prompt = self
            .prompt_manager
            .build_system_prompt(extensions_info, self.frontend_instructions.clone());

        // Handle toolshim if enabled
        let mut toolshim_tools = vec![];
        if self.provider.get_model_config().toolshim {
            // If tool interpretation is enabled, modify the system prompt
            system_prompt = modify_system_prompt_for_tool_json(&system_prompt, &tools);
            // Make a copy of tools before emptying
            toolshim_tools = tools.clone();
            // Empty the tools vector for provider completion
            tools = vec![];
        }

        Ok((tools, toolshim_tools, system_prompt))
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

        // Setup tools and prompt
        let (mut tools, toolshim_tools, mut system_prompt) = self
            .prepare_tools_and_prompt(&mut extension_manager)
            .await?;

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
                match self.provider().complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await {
                    Ok((mut response, usage)) => {
                        // Post-process / structure the response only if tool interpretation is enabled
                        if self.provider.get_model_config().toolshim {
                            let interpreter = OllamaInterpreter::new()
                                .map_err(|e| anyhow::anyhow!("Failed to create OllamaInterpreter: {}", e))?;

                            response = augment_message_with_tool_calls(&interpreter, response, &toolshim_tools).await?;
                        }

                        // Update session metrics
                        if let Some(session_config) = session.clone() {
                            update_session_metrics(session_config, &usage, messages.len()).await?;
                        }

                        // Reset truncation attempt
                        truncation_attempt = 0;

                        // Process the response and tool requests
                        let (filtered_response, message_tool_response, should_break) =
                            process_provider_response(self, &response, &mut extension_manager, config, &mut tools, &mut system_prompt).await?;

                        // Yield the filtered response
                        yield filtered_response.clone();

                        if !message_tool_response.content.is_empty() {
                            yield message_tool_response.clone();
                        }

                        if should_break {
                            break;
                        }

                        // Update messages for next iteration
                        messages.push(response);
                        messages.push(message_tool_response);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        let should_continue = handle_truncation_error(
                            self,
                            &mut messages,
                            &mut truncation_attempt,
                            &system_prompt,
                            &mut tools
                        ).await?;

                        if !should_continue {
                            yield Message::assistant().with_text("Error: Context length exceeds limits even after multiple attempts to truncate. Please start a new session with fresh context and try again.");
                            break;
                        }

                        // Re-acquire the lock if it was dropped during truncation
                        extension_manager = self.extension_manager.lock().await;
                    },
                    Err(e) => {
                        let error_message = create_error_response(&e);
                        yield error_message;
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
}
