use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::Agent;
use crate::agents::capabilities::{Capabilities, ResourceItem};
use crate::agents::system::{SystemConfig, SystemError, SystemResult};
use crate::conversation::Conversation;
use crate::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::providers::base::ProviderUsage;
use crate::register_agent;
use crate::token_counter::TokenCounter;
use mcp_core::{ Tool};
use serde_json::Value;

/// Agent impl. that truncates oldest messages when payload over LLM ctx-limit
pub struct TruncateAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
}

impl TruncateAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter,
        }
    }

    async fn enforce_ctx_limit(
        &self,
        system_prompt: &str,
        tools: &[Tool],
        messages: &[Message],
        target_limit: usize,
        resource_items: &mut [ResourceItem],
    ) -> SystemResult<Vec<Message>> {
        // Flatten all resource content into a vector of strings
        let resources: Vec<String> = resource_items
            .iter()
            .map(|item| item.content.clone())
            .collect();

        let approx_count =
            self.token_counter
                .count_everything(system_prompt, messages, tools, &resources);

        let mut new_messages = messages.to_vec();

        if approx_count > target_limit {
            new_messages = self.drop_messages(messages, approx_count, target_limit);
            if new_messages.is_empty() {
                return Err(SystemError::ContextLimit);
            }
        }

        Ok(new_messages)
    }

    fn text_content_size(&self, message: Option<&Message>) -> usize {
        if let Some(msg) = message {
            let mut approx_count = 0;
            for content in msg.content.iter() {
                if let Some(content_text) = content.as_text() {
                    approx_count += self.token_counter.count_tokens(content_text);
                }
            }
            return approx_count;
        }

        0
    }

    fn drop_messages(
        &self,
        messages: &[Message],
        approx_count: usize,
        target_limit: usize,
    ) -> Vec<Message> {
        debug!(
            "[WARNING] Conversation history has size: {} exceeding the token budget of {}. \
            Dropping oldest messages.",
            approx_count,
            approx_count - target_limit
        );

        if let Ok(mut conversation) = Conversation::parse(messages, &self.token_counter) {
            if let Some(last_interaction) = conversation.interactions.last() {
                if last_interaction.token_count > target_limit {
                    conversation.interactions.pop();
                }
            }

            let mut current_tokens = approx_count;
            let mut keep = 0;
            for (i, interaction) in conversation.interactions.iter().enumerate() {
                keep = i;
                if current_tokens < target_limit {
                    break;
                }
                current_tokens = current_tokens.saturating_sub(interaction.token_count);
            }

            let final_conv = Conversation::new(conversation.interactions[keep..].to_vec());
            let ret = final_conv.render();
            return ret;
        }

        vec![]
    }
}

#[async_trait]
impl Agent for TruncateAgent {
    #[instrument(skip(self, messages), fields(user_message))]
    async fn reply(
        &self,
        messages: &[Message],
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let reply_span = tracing::Span::current();
        let mut capabilities = self.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let system_prompt = capabilities.get_system_prompt().await;
        let estimated_limit = capabilities
            .provider()
            .get_model_config()
            .get_estimated_limit();

        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        let mut messages = self
            .enforce_ctx_limit(
                &system_prompt,
                &tools,
                messages,
                estimated_limit,
                &mut capabilities.get_resources().await?,
            )
            .await?;

        Ok(Box::pin(async_stream::try_stream! {
            let _reply_guard = reply_span.enter();

            loop {
                messages = self
                    .enforce_ctx_limit(
                        &system_prompt,
                        &tools,
                        &messages,
                        estimated_limit,
                        &mut capabilities.get_resources().await?,
                    )
                    .await?;

                // Get completion from provider
                let (response, usage) = capabilities.provider().complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await?;
                capabilities.record_usage(usage).await;

                // Yield the assistant's response
                yield response.clone();

                tokio::task::yield_now().await;

                // First collect any tool requests
                let tool_requests: Vec<&ToolRequest> = response.content
                    .iter()
                    .filter_map(|content| content.as_tool_request())
                    .collect();

                if tool_requests.is_empty() {
                    break;
                }

                // Then dispatch each in parallel
                let futures: Vec<_> = tool_requests
                    .iter()
                    .filter_map(|request| request.tool_call.clone().ok())
                    .map(|tool_call| capabilities.dispatch_tool_call(tool_call))
                    .collect();

                // Process all the futures in parallel but wait until all are finished
                let outputs = futures::future::join_all(futures).await;

                // Create a message with the responses
                let mut message_tool_response = Message::user();
                // Now combine these into MessageContent::ToolResponse using the original ID
                for (request, output) in tool_requests.iter().zip(outputs.into_iter()) {
                    message_tool_response = message_tool_response.with_tool_response(
                        request.id.clone(),
                        output,
                    );
                }

                let tool_resp_size = self.text_content_size(
                    Some(&message_tool_response),
                );
                if tool_resp_size < estimated_limit {
                    yield message_tool_response.clone();
                }

                messages.push(response);
                messages.push(message_tool_response);

                messages = self
                    .enforce_ctx_limit(
                        &system_prompt,
                        &tools,
                        &messages,
                        estimated_limit,
                        &mut capabilities.get_resources().await?,
                    )
                    .await?
            }
        }))
    }

    async fn add_system(&mut self, system: SystemConfig) -> SystemResult<()> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_system(system).await
    }

    async fn remove_system(&mut self, name: &str) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities
            .remove_system(name)
            .await
            .expect("Failed to remove system");
    }

    async fn list_systems(&self) -> Vec<String> {
        let capabilities = self.capabilities.lock().await;
        capabilities
            .list_systems()
            .await
            .expect("Failed to list systems")
    }

    async fn passthrough(&self, _system: &str, _request: Value) -> SystemResult<Value> {
        // TODO implement
        Ok(Value::Null)
    }

    async fn usage(&self) -> Vec<ProviderUsage> {
        let capabilities = self.capabilities.lock().await;
        capabilities.get_usage().await
    }
}

register_agent!("truncate", TruncateAgent);

#[cfg(test)]
mod tests {
    use crate::agents::truncate::TruncateAgent;
    use crate::message::Message;
    use crate::providers::base::{Provider, ProviderUsage, Usage};
    use crate::providers::configs::ModelConfig;
    use mcp_core::{Content, Tool};
    use std::iter;

    // Mock Provider implementation for testing
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn get_model_config(&self) -> &ModelConfig {
            &self.model_config
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage)> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }

        fn get_usage(&self, _data: &serde_json::Value) -> anyhow::Result<Usage> {
            Ok(Usage::new(None, None, None))
        }
    }

    const SMALL_MESSAGE: &str = "This is a test, this is just a test, this is only a test.\n";
    const TOOL_MESSAGE: &str = "I am a tool response. Tooly Tool Tool McToolFace III.\n";

    async fn call_enforce_ctx_limit(conversation: &[Message], target_limit: Option<usize>) -> anyhow::Result<Vec<Message>> {
        let mock_model_config =
            ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());
        let provider = Box::new(MockProvider {
            model_config: mock_model_config,
        });
        let agent = TruncateAgent::new(provider);

        let mut capabilities = agent.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let system_prompt = capabilities.get_system_prompt().await;
        let estimated_limit = if let Some(limit) = target_limit {
            limit
        } else {
            capabilities
                .provider()
                .get_model_config()
                .get_estimated_limit()
        };

        let approx_count =
            agent.token_counter
                .count_everything(&system_prompt, conversation, &tools, &[]);

        let messages = agent.drop_messages(conversation, approx_count, estimated_limit);

        Ok(messages)
    }

    fn create_basic_valid_conversation(
        interactions_count: usize,
        tool_use_limit: usize,
    ) -> Vec<Message> {
        let mut conv = Vec::<Message>::new();

        for i in 0..interactions_count {
            let usr_msg = Message::user().with_text(format!("{:?}{}", SMALL_MESSAGE, i));
            let assistant_msg_txt = format!("{:?}{}", SMALL_MESSAGE, i + 1);
            let assistant_msg = Message::assistant().with_text(assistant_msg_txt);
            conv.push(usr_msg);
            conv.push(assistant_msg);

            if tool_use_limit > 0 {
                // let timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();
                // let rand_tool_use_count = timestamp % tool_use_limit as u128;

                for i in 0..tool_use_limit {
                    let tool_output = format!("{:?}{}", TOOL_MESSAGE, i);
                    let content = vec![Content::text(tool_output)];
                    conv.push(Message::user().with_tool_response("id:0", Ok(content)));

                    let assistant_summary = format!("{:?}{}", SMALL_MESSAGE, i + 1);
                    conv.push(Message::assistant().with_text(assistant_summary));
                }
            }
        }

        conv
    }
    #[tokio::test]
    async fn test_simple_conversation_no_truncation() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(1, 0);
        let messages = call_enforce_ctx_limit(&conversation, None).await?;
        assert_eq!(messages.len(), conversation.len());
        Ok(())
    }
    #[tokio::test]
    async fn test_truncation_when_conversation_history_too_big() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(5000, 0);
        let messages = call_enforce_ctx_limit(&*conversation, None).await?;
        assert!(conversation.len() > messages.len(), "Conversation should be truncated");
        assert!(!messages.is_empty(), "Messages should not be empty");
        Ok(())
    }

    #[tokio::test]
    async fn test_truncation_when_single_user_message_too_big() -> anyhow::Result<()> {
        let oversized_message: String = iter::repeat(SMALL_MESSAGE)
            .take(10000)
            .collect::<Vec<&str>>()
            .join("");
        let mut conversation = create_basic_valid_conversation(3, 0);
        conversation.push(Message::user().with_text(oversized_message));

        let result = call_enforce_ctx_limit(&*conversation, None).await;

        assert_eq!(result?.len(), conversation.len() - 1, "Conversation should be truncated");
        Ok(())
    }

    #[tokio::test]
    async fn test_truncation_when_too_many_tool_responses() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(3, 3);
        let messages = call_enforce_ctx_limit(&*conversation, Some(500)).await?;

        assert!(conversation.len() > messages.len(), "Conversation should be truncated");
        assert!(!messages.is_empty(), "Messages should not be empty");

        Ok(())
    }
}
