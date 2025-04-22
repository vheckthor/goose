use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use goose::agents::Agent;
use goose::message::{Message, MessageContent, ToolRequest};
use mcp_core::{Content, ToolResult};

/// Represents the state of the agent's reply process
pub struct AgentReplyState {
    /// Agent reference
    agent: *mut Agent,
    /// Current set of messages in the conversation
    messages: Vec<Message>,
    /// Pending tool requests
    pending_tool_requests: Vec<ToolRequest>,
    /// Current response from the model
    current_response: Option<Message>,
    /// State indicating whether a tool call is in progress
    tool_call_in_progress: bool,
    /// Current truncation attempt count
    truncation_attempt: usize,
}

/// Possible results from executing a reply step
pub enum StepResult {
    /// The reply is complete
    Complete(Message),
    /// A tool call is needed
    ToolCallNeeded(ToolRequest),
}

impl AgentReplyState {
    /// Create a new agent reply state
    pub async fn new(agent: &mut Agent, messages: Vec<Message>) -> Result<Self> {
        Ok(Self {
            agent,
            messages,
            pending_tool_requests: Vec::new(),
            current_response: None,
            tool_call_in_progress: false,
            truncation_attempt: 0,
        })
    }

    /// Execute one step of the reply process - non-recursive implementation
    pub async fn step(&mut self) -> Result<StepResult> {
        // If there are pending tool requests, return the next one
        if !self.pending_tool_requests.is_empty() {
            let tool_request = self.pending_tool_requests.remove(0);
            return Ok(StepResult::ToolCallNeeded(tool_request));
        }

        // Get a reference to the agent
        let agent = unsafe { &mut *self.agent };

        if !self.tool_call_in_progress {
            // Starting a new interaction
            // Try to get a response from the model
            match try_get_model_response(agent, &self.messages, self.truncation_attempt).await {
                Ok((response, tool_requests)) => {
                    // Reset truncation attempt on success
                    self.truncation_attempt = 0;

                    // Store the response
                    self.current_response = Some(response.clone());

                    if tool_requests.is_empty() {
                        // No tool requests, we're done
                        Ok(StepResult::Complete(response))
                    } else {
                        // Store all tool requests for processing
                        self.pending_tool_requests = tool_requests;

                        // Set flag indicating we're in the middle of tool calls
                        self.tool_call_in_progress = true;

                        // Return the first tool request
                        let tool_request = self.pending_tool_requests.remove(0);
                        Ok(StepResult::ToolCallNeeded(tool_request))
                    }
                }
                Err(TryResponseError::ContextLengthExceeded) => {
                    // Increment truncation attempt
                    self.truncation_attempt += 1;

                    // Check if we've tried too many times
                    if self.truncation_attempt > 3 {
                        return Err(anyhow!("Context length exceeds limits even after multiple attempts to truncate"));
                    }

                    // Try to reduce the message history
                    if self.messages.len() > 4 {
                        // Keep only the last few messages
                        let len = self.messages.len();
                        let new_start = (len / 2).max(2); // Keep at least half or last 2 messages
                        self.messages = self.messages.split_off(new_start);
                    }

                    // Return a status message - next step will try again with shorter history
                    Ok(StepResult::Complete(Message::assistant().with_text(
                        "Context length exceeded. Truncated message history and trying again.",
                    )))
                }
                Err(TryResponseError::Other(e)) => {
                    // Other errors just get passed through
                    Err(e)
                }
            }
        } else {
            // We've processed all pending tool requests, get final response
            if self.pending_tool_requests.is_empty() {
                // Create a composite message with all tool responses
                if let Some(response) = self.current_response.take() {
                    // Add the response to the conversation
                    self.messages.push(response);

                    // Reset the tool call state
                    self.tool_call_in_progress = false;

                    // Start a new reply that will pick up with the new message history
                    Ok(StepResult::Complete(
                        Message::assistant().with_text("Processing tool results..."),
                    ))
                } else {
                    // Should never reach here
                    Err(anyhow!(
                        "Invalid state: tool_call_in_progress but no current response"
                    ))
                }
            } else {
                // Should never reach here
                Err(anyhow!(
                    "Invalid state: tool_call_in_progress but pending tool requests are empty"
                ))
            }
        }
    }

    /// Apply a tool result to the reply state
    pub async fn apply_tool_result(
        &mut self,
        id: String,
        result: ToolResult<Vec<Content>>,
    ) -> Result<()> {
        // Add the tool result to the current message history
        let tool_response = Message::user().with_tool_response(id, result);
        self.messages.push(tool_response);

        // If this was the last pending tool request, proceed to the next step
        if self.pending_tool_requests.is_empty() {
            // Reset the tool call state to process the next response
            self.tool_call_in_progress = false;
        }

        Ok(())
    }
}

// Error type for the try_get_model_response function
enum TryResponseError {
    ContextLengthExceeded,
    Other(anyhow::Error),
}

// Helper function to get a response from the model
async fn try_get_model_response(
    agent: &mut Agent,
    messages: &[Message],
    _truncation_attempt: usize,
) -> Result<(Message, Vec<ToolRequest>), TryResponseError> {
    // Start a reply stream
    let reply_result = agent.reply(messages, None).await;

    // Handle reply stream creation errors
    let mut reply_stream = match reply_result {
        Ok(stream) => stream,
        Err(e) => {
            if e.to_string().contains("Context length exceeds") {
                return Err(TryResponseError::ContextLengthExceeded);
            } else {
                return Err(TryResponseError::Other(anyhow!(
                    "Error creating reply stream: {}",
                    e
                )));
            }
        }
    };

    // Get the first response
    match reply_stream.try_next().await {
        Ok(Some(response)) => {
            // Extract tool requests from the response
            let tool_requests: Vec<ToolRequest> = response
                .content
                .iter()
                .filter_map(|content| {
                    if let MessageContent::ToolRequest(req) = content {
                        Some(req.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Filter content to create a message without tool requests
            let filtered_content = response
                .content
                .iter()
                .filter(|c| !matches!(c, MessageContent::ToolRequest(_)))
                .cloned()
                .collect();

            let filtered_message = Message {
                role: response.role.clone(),
                created: response.created,
                content: filtered_content,
            };

            Ok((filtered_message, tool_requests))
        }
        Ok(None) => {
            // No response from the stream, this is unusual
            Err(TryResponseError::Other(anyhow!(
                "No response received from agent"
            )))
        }
        Err(e) => Err(TryResponseError::Other(anyhow!(
            "Error getting response: {}",
            e
        ))),
    }
}

// Ensure cleanup on drop
impl Drop for AgentReplyState {
    fn drop(&mut self) {
        // Nothing to clean up
    }
}
