use std::sync::Arc;

use anyhow::Result;

use crate::agents::agent::Agent;
use crate::message::{Message, ToolRequest};
use crate::permission::{
    detect_read_only_tools, ToolPermissionStore,
};
use crate::providers::base::Provider;

use mcp_core::{tool::Tool, ToolCall};

/// Check permissions for tool requests and categorize them based on required confirmation
pub async fn check_tool_permissions<'a>(
    agent: &Agent,
    requests: &[&'a ToolRequest],
    tools_with_readonly_annotation: &[String],
    tools_without_annotation: &[String],
) -> Result<(
    Vec<(String, ToolCall)>,
    Vec<&'a ToolRequest>,
    Vec<&'a ToolRequest>,
)> {
    let mut approved_tools = Vec::new();
    let mut needs_confirmation = Vec::new();
    let mut llm_detect_candidates = Vec::new();

    let store = ToolPermissionStore::load()?;

    for request in requests {
        if let Ok(tool_call) = &request.tool_call {
            // Check if the tool has a read-only annotation
            if tools_with_readonly_annotation.contains(&tool_call.name) {
                approved_tools.push((request.id.clone(), tool_call.clone()));
            } else if let Some(allowed) = store.check_permission(request) {
                if allowed {
                    // Tool has been previously approved
                    approved_tools.push((request.id.clone(), tool_call.clone()));
                } else {
                    // Tool has been previously denied
                    if tools_without_annotation.contains(&tool_call.name) {
                        llm_detect_candidates.push(*request);
                    }
                    needs_confirmation.push(*request);
                }
            } else {
                // No previous decision for this tool
                if tools_without_annotation.contains(&tool_call.name) {
                    llm_detect_candidates.push(*request);
                }
                needs_confirmation.push(*request);
            }
        }
    }

    Ok((approved_tools, needs_confirmation, llm_detect_candidates))
}

/// Handle confirmation for a tool request
pub async fn handle_tool_confirmation(
    agent: &Agent,
    request: &ToolRequest,
) -> Result<Option<Tool>> {
    if let Ok(tool_call) = &request.tool_call {
        let confirmation = Message::user().with_tool_confirmation_request(
            request.id.clone(),
            tool_call.name.clone(),
            tool_call.arguments.clone(),
            Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
        );

        // In the actual implementation, this would yield the confirmation and wait for a response
        // For now, we'll just return None to indicate no tool should be executed

        // Wait for confirmation response through the channel
        // This would be implemented in the Agent struct

        // Return None for now
        return Ok(None);
    }

    Ok(None)
}

/// Use the LLM to detect which tools are read-only and can be run without confirmation
pub async fn detect_safe_tools(
    provider: Arc<dyn Provider>,
    candidates: &[&ToolRequest],
) -> Vec<String> {
    // Call the detect_read_only_tools function from the permission module
    detect_read_only_tools(provider, candidates.to_vec()).await
}
