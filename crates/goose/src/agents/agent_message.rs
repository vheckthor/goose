use anyhow::Result;
use tracing::error;

use crate::agents::agent::Agent;
use crate::agents::agent_tool_processing::{
    categorize_tool_requests, process_extension_tools, process_frontend_tools,
    process_standard_tools,
};
use crate::agents::extension_manager::ExtensionManager;
use crate::config::Config;
use crate::message::{Message, ToolRequest};
use crate::providers::errors::ProviderError;
use crate::session;

use mcp_core::tool::Tool;

/// Process the response from the provider and handle any tool requests.
///
/// This function:
/// 1. Filters out frontend tool requests from the response
/// 2. Categorizes tool requests (frontend, extension, standard)
/// 3. Processes each category of tool requests according to the current goose_mode
/// 4. Generates tool responses and updates the system state as needed
///
/// # Arguments
///
/// * `agent` - Reference to the Agent instance
/// * `response` - The original response from the provider
/// * `extension_manager` - Mutable reference to the extension manager
/// * `config` - Reference to the global configuration
/// * `tools` - Mutable reference to the tools vector
/// * `system_prompt` - Reference to the current system prompt
///
/// # Returns
///
/// A tuple containing:
/// * `filtered_response`: The provider's response with frontend tool requests filtered out
/// * `message_tool_response`: A message containing all tool responses
/// * `should_break`: A boolean indicating whether to break the reply loop:
///   - `true` if there are no tool requests or all tool requests have been handled
///   - `false` if there are more tool requests to process and the loop should continue
///
/// # Errors
///
/// Returns an error if tool processing fails.
pub async fn process_provider_response(
    agent: &Agent,
    response: &Message,
    extension_manager: &mut ExtensionManager,
    config: &Config,
    tools: &mut Vec<Tool>,
    system_prompt: &str,
) -> Result<(Message, Message, bool)> {
    // Get the goose_mode from config
    let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

    // Filter out frontend tool requests
    let filtered_response = Message {
        role: response.role.clone(),
        created: response.created,
        content: response
            .content
            .iter()
            .filter(|c| {
                if let Some(req) = c.as_tool_request() {
                    if let Ok(tool_call) = &req.tool_call {
                        return !agent.is_frontend_tool(&tool_call.name);
                    }
                }
                true
            })
            .cloned()
            .collect(),
    };

    // Collect tool requests
    let tool_requests: Vec<&ToolRequest> = response
        .content
        .iter()
        .filter_map(|content| content.as_tool_request())
        .collect();

    if tool_requests.is_empty() {
        return Ok((filtered_response, Message::user(), true));
    }

    // Categorize tool requests
    let (frontend_requests, extension_requests, standard_requests) =
        categorize_tool_requests(&tool_requests);

    // Process frontend tools
    let mut message_tool_response = process_frontend_tools(agent, &frontend_requests).await?;

    // Process extension tools
    let extension_response = process_extension_tools(
        agent,
        &extension_requests,
        extension_manager,
        system_prompt,
        tools,
    )
    .await?;

    // Merge the responses
    for content in extension_response.content {
        message_tool_response.content.push(content);
    }

    // Process standard tools based on goose_mode
    let standard_response =
        process_standard_tools(agent, &standard_requests, extension_manager, &goose_mode).await?;

    // Merge the responses
    for content in standard_response.content {
        message_tool_response.content.push(content);
    }

    Ok((filtered_response, message_tool_response, false))
}

/// Update session metrics after a response
pub async fn update_session_metrics(
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

/// Create an error response message
pub fn create_error_response(error: &ProviderError) -> Message {
    error!("Error: {}", error);

    Message::assistant().with_text(format!(
        "Ran into this error: {}.\n\nPlease retry if you think this is a transient or recoverable error.",
        error
    ))
}
