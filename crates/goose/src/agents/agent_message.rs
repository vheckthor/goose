use anyhow::Result;
use futures::future::join_all;
use tracing::error;

use crate::agents::agent::Agent;
use crate::agents::agent_permission::{
    check_tool_permissions, detect_safe_tools, handle_tool_confirmation,
};
use crate::agents::agent_tool_processing::{
    categorize_tool_requests, create_tool_future, process_extension_tools, process_frontend_tools,
    process_standard_tools,
};
use crate::agents::extension_manager::ExtensionManager;
use crate::config::Config;
use crate::message::{Message, ToolRequest};
use crate::providers::errors::ProviderError;
use crate::session;

use mcp_core::{tool::Tool, Content};

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
    system_prompt: &mut String,
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
        categorize_tool_requests(agent, &tool_requests);

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
    if goose_mode == "auto" || goose_mode == "chat" {
        let standard_response =
            process_standard_tools(agent, &standard_requests, extension_manager, &goose_mode)
                .await?;

        // Merge the responses
        for content in standard_response.content {
            message_tool_response.content.push(content);
        }
    } else if goose_mode == "approve" || goose_mode == "smart_approve" {
        // Handle approve and smart_approve modes using permission functions

        // Get tools with annotations
        let (tools_with_readonly_annotation, tools_without_annotation): (Vec<String>, Vec<String>) =
            tools.iter().fold((vec![], vec![]), |mut acc, tool| {
                match &tool.annotations {
                    Some(annotations) => {
                        if annotations.read_only_hint {
                            acc.0.push(tool.name.clone());
                        } else {
                            acc.1.push(tool.name.clone());
                        }
                    }
                    None => {
                        acc.1.push(tool.name.clone());
                    }
                }
                acc
            });

        // Check permissions for standard tools
        let (approved_tools, needs_confirmation, llm_detect_candidates) = check_tool_permissions(
            agent,
            &standard_requests,
            &tools_with_readonly_annotation,
            &tools_without_annotation,
        )
        .await?;

        let mut tool_futures = Vec::new();

        // Process pre-approved tools
        for (request_id, tool_call) in approved_tools {
            let is_frontend_tool = agent.is_frontend_tool(&tool_call.name);
            let tool_future =
                create_tool_future(extension_manager, tool_call, is_frontend_tool, request_id);
            tool_futures.push(tool_future);
        }

        // Use LLM to detect safe tools if in smart_approve mode
        let mut detected_read_only_tools = Vec::new();
        if goose_mode == "smart_approve" && !llm_detect_candidates.is_empty() {
            detected_read_only_tools =
                detect_safe_tools(agent.provider(), &llm_detect_candidates).await;
        }

        // Process tools that need confirmation
        for request in needs_confirmation {
            if let Ok(tool_call) = &request.tool_call {
                let is_frontend_tool = agent.is_frontend_tool(&tool_call.name);

                // Skip confirmation if the tool is detected as read-only
                if detected_read_only_tools.contains(&tool_call.name) {
                    let tool_future = create_tool_future(
                        extension_manager,
                        tool_call.clone(),
                        is_frontend_tool,
                        request.id.clone(),
                    );
                    tool_futures.push(tool_future);
                } else {
                    // Handle confirmation
                    if let Ok(Some(tool)) = handle_tool_confirmation(agent, request).await {
                        let tool_future = create_tool_future(
                            extension_manager,
                            tool,
                            is_frontend_tool,
                            request.id.clone(),
                        );
                        tool_futures.push(tool_future);
                    } else {
                        // User declined - add declined response
                        message_tool_response = message_tool_response.with_tool_response(
                            request.id.clone(),
                            Ok(vec![Content::text(
                                "The user has declined to run this tool. \
                                DO NOT attempt to call this tool again. \
                                If there are no alternative methods to proceed, clearly explain the situation and STOP.")]),
                        );
                    }
                }
            }
        }

        // Wait for all tool calls to complete
        let results = join_all(tool_futures).await;
        for (request_id, output) in results {
            message_tool_response = message_tool_response.with_tool_response(request_id, output);
        }
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
