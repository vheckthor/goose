use anyhow::{anyhow, Result};

use crate::agents::agent::Agent;
use crate::agents::extension_manager::ExtensionManager;
use crate::config::ExtensionConfigManager;
use crate::message::ToolRequest;

use mcp_core::{tool::Tool, Content, ToolError};

/// Handle the installation of an extension
pub async fn handle_extension_installation(
    request: &ToolRequest,
    extension_manager: &mut ExtensionManager,
) -> Result<(String, Result<Vec<Content>, ToolError>)> {
    if let Ok(tool_call) = &request.tool_call {
        let extension_name = tool_call
            .arguments
            .get("extension_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let result = enable_extension(extension_manager, extension_name, request.id.clone()).await;
        return Ok(result);
    }

    Err(anyhow!("Invalid tool call for extension installation"))
}

/// Enable an extension by name
pub async fn enable_extension(
    extension_manager: &mut ExtensionManager,
    extension_name: String,
    request_id: String,
) -> (String, Result<Vec<Content>, ToolError>) {
    let config = match ExtensionConfigManager::get_config_by_name(&extension_name) {
        Ok(Some(config)) => config,
        Ok(None) => {
            return (
                request_id,
                Err(ToolError::ExecutionError(format!(
                    "Extension '{}' not found. Please check the extension name and try again.",
                    extension_name
                ))),
            )
        }
        Err(e) => {
            return (
                request_id,
                Err(ToolError::ExecutionError(format!(
                    "Failed to get extension config: {}",
                    e
                ))),
            )
        }
    };

    let result = extension_manager
        .add_extension(config)
        .await
        .map(|_| {
            vec![Content::text(format!(
                "The extension '{}' has been installed successfully",
                extension_name
            ))]
        })
        .map_err(|e| ToolError::ExecutionError(e.to_string()));

    (request_id, result)
}

/// Update system prompt and tools after extension changes
pub async fn update_after_extension_changes(
    agent: &Agent,
    extension_manager: &mut ExtensionManager,
) -> Result<(String, Vec<Tool>)> {
    let extensions_info = extension_manager.get_extensions_info().await;

    // Build the updated system prompt using the agent's prompt manager
    let system_prompt = agent
        .prompt_manager
        .build_system_prompt(extensions_info, agent.frontend_instructions.clone());

    // Get the updated tools
    let tools = extension_manager.get_prefixed_tools().await?;

    Ok((system_prompt, tools))
}
