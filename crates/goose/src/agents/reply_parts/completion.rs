use anyhow::Result;

use crate::agents::{platform_tools, ExtensionManager};
use crate::message::Message;
use crate::providers::base::ProviderUsage;
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use crate::session;
use mcp_core::tool::Tool;

use super::super::agent::Agent;

impl Agent {
    /// Prepares tools and system prompt for a provider request
    pub(crate) async fn prepare_tools_and_prompt(
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

    /// Generate a response from the LLM provider
    /// Handles toolshim transformations if needed
    pub(crate) async fn generate_response_from_provider(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = self.provider.get_model_config();

        // Call the provider to get a response
        let (mut response, usage) = self
            .provider()
            .complete(system_prompt, messages, tools)
            .await?;

        // Post-process / structure the response only if tool interpretation is enabled
        if config.toolshim {
            let interpreter = OllamaInterpreter::new().map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to create OllamaInterpreter: {}", e))
            })?;

            response = augment_message_with_tool_calls(&interpreter, response, toolshim_tools)
                .await
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to augment message: {}", e))
                })?;
        }

        Ok((response, usage))
    }

    /// Update session metrics after a response
    pub(crate) async fn update_session_metrics(
        session_config: crate::agents::types::SessionConfig,
        usage: &crate::providers::base::ProviderUsage,
        message_count: usize,
    ) -> Result<()> {
        let session_file = session::get_path(session_config.id);
        let mut metadata = session::read_metadata(&session_file)?;

        metadata.working_dir = session_config.working_dir.clone();
        metadata.total_tokens = usage.usage.total_tokens;
        metadata.input_tokens = usage.usage.input_tokens;
        metadata.output_tokens = usage.usage.output_tokens;
        metadata.message_count = message_count + 1;

        session::update_metadata(&session_file, &metadata).await?;

        Ok(())
    }
}
