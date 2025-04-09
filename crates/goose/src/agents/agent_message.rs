use anyhow::{anyhow, Result};
use tracing::{error, warn};

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
use crate::truncate::{truncate_messages, OldestFirstTruncation};

use mcp_core::{tool::Tool};

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// Process the response from the provider and handle tool requests
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

/// Handle truncation when context length is exceeded
pub async fn handle_truncation_error(
    agent: &Agent,
    messages: &mut Vec<Message>,
    truncation_attempt: &mut usize,
    system_prompt: &str,
    tools: &mut Vec<Tool>,
) -> Result<bool> {
    if *truncation_attempt >= MAX_TRUNCATION_ATTEMPTS {
        return Ok(false);
    }

    *truncation_attempt += 1;
    warn!(
        "Context length exceeded. Truncation Attempt: {}/{}.",
        truncation_attempt, MAX_TRUNCATION_ATTEMPTS
    );

    // Decay the estimate factor as we make more truncation attempts
    let estimate_factor: f32 = ESTIMATE_FACTOR_DECAY.powi(*truncation_attempt as i32);

    // In the actual implementation, this would call agent.truncate_messages
    // For now, we'll just simulate truncation by removing the oldest messages

    // Model's actual context limit (placeholder)
    let context_limit = 8192;

    // Our conservative estimate of the target context limit
    let context_limit = (context_limit as f32 * estimate_factor) as usize;

    // Take into account the system prompt and tools
    let system_prompt_token_count = system_prompt.len() / 4; // Rough approximation
    let tools_token_count = tools.len() * 100; // Rough approximation

    let remaining_tokens = context_limit
        .checked_sub(system_prompt_token_count)
        .and_then(|remaining| remaining.checked_sub(tools_token_count))
        .ok_or_else(|| anyhow!("System prompt and tools exceed estimated context limit"))?;

    // Calculate token counts for messages (placeholder)
    let mut token_counts: Vec<usize> = messages
        .iter()
        .map(|msg| msg.content.len() * 2) // Rough approximation
        .collect();

    // Truncate messages
    truncate_messages(
        messages,
        &mut token_counts,
        remaining_tokens,
        &OldestFirstTruncation,
    )?;

    Ok(true)
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
