use anyhow::Result;
use tracing::{debug, instrument};

use crate::agents::agent::Agent;
use crate::agents::agent_extension::{
    handle_extension_installation, update_after_extension_changes,
};
use crate::agents::extension_manager::ExtensionManager;
use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::message::{Message, ToolRequest};

use mcp_core::{tool::Tool, tool::ToolCall, Content, ToolError};

use super::platform_tools::PLATFORM_ENABLE_EXTENSION_TOOL_NAME;

/// Categorizes tool requests into different types: frontend, extension, and standard
pub fn categorize_tool_requests<'a>(
    agent: &Agent,
    tool_requests: &'a [&'a ToolRequest],
) -> (
    Vec<&'a ToolRequest>,
    Vec<&'a ToolRequest>,
    Vec<&'a ToolRequest>,
) {
    let mut frontend_requests = Vec::new();
    let mut extension_requests = Vec::new();
    let mut standard_requests = Vec::new();

    for request in tool_requests {
        if let Ok(tool_call) = &request.tool_call {
            if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME
                || tool_call.name == PLATFORM_ENABLE_EXTENSION_TOOL_NAME
            {
                extension_requests.push(*request);
            } else if agent.is_frontend_tool(&tool_call.name) {
                frontend_requests.push(*request);
            } else {
                standard_requests.push(*request);
            }
        }
    }

    (frontend_requests, extension_requests, standard_requests)
}

/// Process frontend tool requests
///
/// Returns a tuple of:
/// - The message with tool responses
/// - A vector of frontend tool requests that need to be yielded to the user
pub async fn process_frontend_tools(
    agent: &Agent,
    frontend_requests: &[&ToolRequest],
) -> Result<(Message, Vec<(String, ToolCall)>)> {
    let message_tool_response = Message::user();
    let mut frontend_tool_requests = Vec::new();

    for request in frontend_requests {
        if let Ok(tool_call) = &request.tool_call {
            if agent.is_frontend_tool(&tool_call.name) {
                // Add this frontend tool request to the list to be yielded
                frontend_tool_requests.push((request.id.clone(), tool_call.clone()));
            }
        }
    }

    Ok((message_tool_response, frontend_tool_requests))
}

/// Process extension-related tool requests (search, enable)
pub async fn process_extension_tools(
    agent: &Agent,
    extension_requests: &[&ToolRequest],
    extension_manager: &mut ExtensionManager,
    system_prompt: &mut String,
    tools: &mut Vec<Tool>,
) -> Result<Message> {
    let mut message_tool_response = Message::user();
    let mut install_results = Vec::new();

    for request in extension_requests {
        if let Ok(tool_call) = &request.tool_call {
            if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
                let result = extension_manager.search_available_extensions().await;
                message_tool_response =
                    message_tool_response.with_tool_response(request.id.clone(), result);
            } else if tool_call.name.contains("enable_extension") {
                let install_result =
                    handle_extension_installation(request, extension_manager).await?;
                install_results.push(install_result);

                // Update system prompt and tools after extension changes
                let (updated_system_prompt, updated_tools) =
                    update_after_extension_changes(agent, extension_manager).await?;

                // Update the system_prompt and tools references
                *system_prompt = updated_system_prompt;
                *tools = updated_tools;
            }
        }
    }

    // Add all installation results to the response
    for (request_id, output) in install_results {
        message_tool_response = message_tool_response.with_tool_response(request_id, output);
    }

    Ok(message_tool_response)
}

/// Process standard tool requests based on the goose_mode
pub async fn process_standard_tools(
    agent: &Agent,
    standard_requests: &[&ToolRequest],
    extension_manager: &ExtensionManager,
    goose_mode: &str,
) -> Result<Message> {
    let mut message_tool_response = Message::user();
    let mut tool_futures = Vec::new();

    for request in standard_requests {
        if let Ok(tool_call) = &request.tool_call {
            let is_frontend_tool = agent.is_frontend_tool(&tool_call.name);

            if goose_mode == "auto" {
                // In auto mode, execute all tool calls without confirmation
                let tool_future = create_tool_future(
                    extension_manager,
                    tool_call.clone(),
                    is_frontend_tool,
                    request.id.clone(),
                );
                tool_futures.push(tool_future);
            } else if goose_mode == "chat" {
                // In chat mode, skip tool calls with a message
                message_tool_response = message_tool_response.with_tool_response(
                    request.id.clone(),
                    Ok(vec![Content::text(
                        "Let the user know the tool call was skipped in Goose chat mode. \
                        DO NOT apologize for skipping the tool call. DO NOT say sorry. \
                        Provide an explanation of what the tool call would do, structured as a \
                        plan for the user. Again, DO NOT apologize. \
                        **Example Plan:**\n \
                        1. **Identify Task Scope** - Determine the purpose and expected outcome.\n \
                        2. **Outline Steps** - Break down the steps.\n \
                        If needed, adjust the explanation based on user preferences or questions.",
                    )]),
                );
            }
            // Note: The "approve" and "smart_approve" modes are handled in process_provider_response
        }
    }

    // Wait for all tool calls to complete
    let results = futures::future::join_all(tool_futures).await;
    for (request_id, output) in results {
        message_tool_response = message_tool_response.with_tool_response(request_id, output);
    }

    Ok(message_tool_response)
}

/// Create a future that executes a tool call
#[instrument(skip(tool_call, extension_manager, request_id), fields(input, output))]
pub async fn create_tool_future(
    extension_manager: &ExtensionManager,
    tool_call: ToolCall,
    is_frontend_tool: bool,
    request_id: String,
) -> (String, Result<Vec<Content>, ToolError>) {
    let result = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
        // Check if the tool is read_resource and handle it separately
        extension_manager
            .read_resource(tool_call.arguments.clone())
            .await
    } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
        extension_manager
            .list_resources(tool_call.arguments.clone())
            .await
    } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
        extension_manager.search_available_extensions().await
    } else if is_frontend_tool {
        // For frontend tools, return an error indicating we need frontend execution
        Err(ToolError::ExecutionError(
            "Frontend tool execution required".to_string(),
        ))
    } else {
        extension_manager
            .dispatch_tool_call(tool_call.clone())
            .await
    };

    debug!(
        "input" = serde_json::to_string(&tool_call).unwrap(),
        "output" = serde_json::to_string(&result).unwrap(),
    );

    (request_id, result)
}
