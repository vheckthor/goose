use std::sync::Arc;

use anyhow::Result;

use crate::agents::agent::Agent;
use crate::message::{Message, ToolRequest};
use crate::permission::{detect_read_only_tools, ToolPermissionStore};
use crate::providers::base::Provider;

use mcp_core::tool::ToolCall;

/// Check permissions for tool requests and categorize them based on required confirmation
pub async fn check_tool_permissions<'a>(
    _agent: &Agent,
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
///
/// Returns a tuple of:
/// - Option<ToolCall>: The tool call to execute if approved, None if denied
/// - Option<(String, Message)>: A confirmation request to yield to the user, if needed
pub async fn handle_tool_confirmation(
    _agent: &Agent,
    request: &ToolRequest,
) -> Result<(Option<ToolCall>, Option<(String, Message)>)> {
    if let Ok(tool_call) = &request.tool_call {
        let confirmation_message = Message::user().with_tool_confirmation_request(
            request.id.clone(),
            tool_call.name.clone(),
            tool_call.arguments.clone(),
            Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
        );

        // Return the tool call and the confirmation message
        // The caller will yield the confirmation message and handle the response
        return Ok((None, Some((request.id.clone(), confirmation_message))));
    }

    Ok((None, None))
}

/// Use the LLM to detect which tools are read-only and can be run without confirmation
pub async fn detect_safe_tools(
    provider: Arc<dyn Provider>,
    candidates: &[&ToolRequest],
) -> Vec<String> {
    // Call the detect_read_only_tools function from the permission module
    detect_read_only_tools(provider, candidates.to_vec()).await
}
