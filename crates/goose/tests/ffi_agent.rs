use std::sync::Arc;

use goose::agents::Agent;
use goose::message::Message;
use goose::model::ModelConfig;
use mcp_core::{Content, ToolResult};

// Import mock tools for testing
mod mock {
    use super::*;
    use goose::providers::base::{Provider, ProviderUsage};
    use goose::providers::errors::{ProviderError, TokenUsage};
    use std::pin::Pin;
    use std::future::Future;
    
    // Simple mock provider for testing
    pub struct MockProvider {
        config: ModelConfig,
    }
    
    impl MockProvider {
        pub fn new(config: ModelConfig) -> Self {
            Self { config }
        }
    }
    
    impl Provider for MockProvider {
        fn complete(
            &self,
            _system_prompt: &str,
            messages: &[Message],
            _tools: &[mcp_core::tool::Tool],
        ) -> Pin<Box<dyn Future<Output = Result<(Message, ProviderUsage), ProviderError>> + Send>> {
            let messages = messages.to_vec();
            Box::pin(async move {
                // Create a simple echo response
                let last_msg = messages.last()
                    .and_then(|m| m.content.first())
                    .and_then(|c| c.as_text())
                    .unwrap_or("");
                
                let response = Message::assistant().with_text(format!("Mock response to: {}", last_msg));
                
                let usage = ProviderUsage {
                    model: "mock".to_string(),
                    usage: TokenUsage {
                        total_tokens: Some(100),
                        input_tokens: Some(50),
                        output_tokens: Some(50),
                    },
                };
                
                Ok((response, usage))
            })
        }
        
        fn provider_id(&self) -> &str {
            "mock"
        }
        
        fn get_model_config(&self) -> ModelConfig {
            self.config.clone()
        }
    }
}

async fn create_agent_with_mock() -> Agent {
    let config = ModelConfig::new("mock");
    let provider = Arc::new(mock::MockProvider::new(config));
    Agent::new(provider)
}

#[tokio::test]
async fn test_step_reply_for_ffi_basic() {
    let agent = create_agent_with_mock().await;
    let messages = vec![Message::user().with_text("Hello, world!")];

    // Initial response test
    let reply = agent
        .step_reply_for_ffi(&messages, vec![], None)
        .await
        .unwrap();
    let response = reply.message;
    let tool_reqs = reply.tool_requests;
    // No tool requests expected in a simple prompt
    assert!(
        tool_reqs.is_empty(),
        "Expected no tool requests, got {:?}",
        tool_reqs
    );
    assert!(
        response.content.as_concat_text().contains("Hello"),
        "Reply should contain greeting: {:?}",
        response.content
    );
}

#[tokio::test]
async fn test_step_reply_for_ffi_with_tool_response() {
    let agent = create_agent_with_mock().await;
    let messages = vec![Message::user().with_text("Please help me with a task")];

    // Test with previous tool response
    let tool_id = "test_tool_123";
    let tool_result: ToolResult<Vec<Content>> = Ok(vec![Content::text("Tool execution result")]);

    let reply = agent
        .step_reply_for_ffi(&messages, vec![(tool_id.to_string(), tool_result)], None)
        .await
        .unwrap();
    let response_with_tool = reply.message;
    let _tool_reqs = reply.tool_requests;
    assert!(
        !response_with_tool.content.as_concat_text().is_empty(),
        "Reply should contain content after tool execution"
    );
}

#[tokio::test]
async fn test_step_reply_for_ffi_conversation_flow() {
    let agent = create_agent_with_mock().await;
    let mut messages = Vec::new();

    // First user message
    messages.push(Message::user().with_text("Hello"));

    // Get first response
    let reply1 = agent
        .step_reply_for_ffi(&messages, vec![], None)
        .await
        .unwrap();
    let response1 = reply1.message;
    let _tool_reqs1 = reply1.tool_requests;
    messages.push(response1.clone());

    // Add next user message
    messages.push(Message::user().with_text("Tell me more"));

    // Get next response
    let reply2 = agent
        .step_reply_for_ffi(&messages, vec![], None)
        .await
        .unwrap();
    let response2 = reply2.message;

    // Verify we're getting different responses in the conversation
    assert_ne!(
        response1.content.as_concat_text(),
        response2.content.as_concat_text(),
        "Responses in conversation should be different"
    );
}

#[tokio::test]
async fn test_step_reply_for_ffi_error_handling() {
    let agent = create_agent_with_mock().await;

    // Create a very large message that should trigger truncation
    let large_message = "x".repeat(500_000);
    let messages = vec![Message::user().with_text(large_message)];

    // This test should either succeed with a valid response after truncation,
    // or return an error if truncation isn't possible
    let result = agent.step_reply_for_ffi(&messages, vec![], None).await;

    match result {
        Ok(reply) => {
            let response = reply.message;
            // If it succeeds, the response should not be empty
            assert!(!response.content.as_concat_text().is_empty());
        }
        Err(e) => {
            // If it fails, the error should mention context length or truncation
            let error_msg = e.to_string().to_lowercase();
            assert!(
                error_msg.contains("context")
                    || error_msg.contains("truncat")
                    || error_msg.contains("length"),
                "Error should be about context length: {}",
                error_msg
            );
        }
    }
}
