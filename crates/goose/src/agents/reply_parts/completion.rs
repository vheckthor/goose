use anyhow::Result;

use crate::message::Message;
use crate::providers::base::ProviderUsage;
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use mcp_core::tool::Tool;

use super::super::agent::Agent;

impl Agent {
    /// Generate a response from the LLM provider
    /// Handles toolshim transformations if needed
    pub async fn generate_response_from_provider(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = self.provider.get_model_config();
        let mut tools_to_use = tools.to_vec();
        let mut system_prompt_to_use = system_prompt.to_string();
        let mut toolshim_tools = vec![];

        // Handle toolshim if configured
        if config.toolshim {
            // If tool interpretation is enabled, modify the system prompt to instruct to return JSON tool requests
            system_prompt_to_use =
                modify_system_prompt_for_tool_json(&system_prompt_to_use, &tools_to_use);
            // make a copy of tools before empty
            toolshim_tools = tools_to_use.clone();
            // pass empty tools vector to provider completion since toolshim will handle tool calls instead
            tools_to_use = vec![];
        }

        // Call the provider to get a response
        let (mut response, usage) = self
            .provider()
            .complete(&system_prompt_to_use, messages, &tools_to_use)
            .await?;

        // Post-process / structure the response only if tool interpretation is enabled
        if config.toolshim {
            let interpreter = OllamaInterpreter::new().map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to create OllamaInterpreter: {}", e))
            })?;

            response = augment_message_with_tool_calls(&interpreter, response, &toolshim_tools)
                .await
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to augment message: {}", e))
                })?;
        }

        Ok((response, usage))
    }
}
