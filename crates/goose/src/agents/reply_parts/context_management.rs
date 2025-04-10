use anyhow::Result;
use tracing::warn;

use crate::message::Message;
use crate::truncate::{truncate_messages, OldestFirstTruncation};
use mcp_core::tool::Tool;

use super::super::agent::Agent;

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

impl Agent {
    /// Handle context truncation logic when context length is exceeded
    /// Returns Some(Message) if an error message should be sent to the user
    /// Returns None if truncation was successful and the loop should continue
    pub(crate) async fn handle_context_truncation(
        &self,
        messages: &mut Vec<Message>,
        truncation_attempt: &mut usize,
        system_prompt: &str,
        tools: &mut Vec<Tool>,
    ) -> Option<Message> {
        if *truncation_attempt >= MAX_TRUNCATION_ATTEMPTS {
            // Create an error message & terminate the stream
            return Some(Message::assistant().with_text(
                "Error: Context length exceeds limits even after multiple attempts to truncate. \
                Please start a new session with fresh context and try again.",
            ));
        }

        *truncation_attempt += 1;
        warn!(
            "Context length exceeded. Truncation Attempt: {}/{}.",
            truncation_attempt, MAX_TRUNCATION_ATTEMPTS
        );

        // Decay the estimate factor as we make more truncation attempts
        // Estimate factor decays like this over time: 0.9, 0.81, 0.729, ...
        let estimate_factor: f32 = ESTIMATE_FACTOR_DECAY.powi(*truncation_attempt as i32);

        // Perform the truncation
        if let Err(err) = self
            .truncate_messages(messages, estimate_factor, system_prompt, tools)
            .await
        {
            return Some(Message::assistant().with_text(
                format!("Error: Unable to truncate messages to stay within context limit. \n\nRan into this error: {}.\n\nPlease start a new session with fresh context and try again.", err)
            ));
        }

        None // Truncation successful, continue the loop
    }

    /// Truncates the messages to fit within the model's context window
    /// Ensures the last message is a user message and removes tool call-response pairs
    pub(crate) async fn truncate_messages(
        &self,
        messages: &mut Vec<Message>,
        estimate_factor: f32,
        system_prompt: &str,
        tools: &mut Vec<Tool>,
    ) -> Result<()> {
        // Model's actual context limit
        let context_limit = self.provider.get_model_config().context_limit();

        // Our conservative estimate of the **target** context limit
        // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
        let context_limit = (context_limit as f32 * estimate_factor) as usize;

        // Take into account the system prompt, and our tools input and subtract that from the
        // remaining context limit
        let system_prompt_token_count = self.token_counter.count_tokens(system_prompt);
        let tools_token_count = self.token_counter.count_tokens_for_tools(tools.as_slice());

        // Check if system prompt + tools exceed our context limit
        let remaining_tokens = context_limit
            .checked_sub(system_prompt_token_count)
            .and_then(|remaining| remaining.checked_sub(tools_token_count))
            .ok_or_else(|| {
                anyhow::anyhow!("System prompt and tools exceed estimated context limit")
            })?;

        let context_limit = remaining_tokens;

        // Calculate current token count of each message, use count_chat_tokens to ensure we
        // capture the full content of the message, include ToolRequests and ToolResponses
        let mut token_counts: Vec<usize> = messages
            .iter()
            .map(|msg| {
                self.token_counter
                    .count_chat_tokens("", std::slice::from_ref(msg), &[])
            })
            .collect();

        truncate_messages(
            messages,
            &mut token_counts,
            context_limit,
            &OldestFirstTruncation,
        )
    }
}
