use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message as WSRawMessage},
    response::Response,
    routing::get,
    Router,
};
use crate::state::AppState;
use goose::message::{Message as GooseMessage, MessageContent};
use mcp_core::role::Role;
use tokio::sync::broadcast;
use futures::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::path::PathBuf;
use goose::{session, agents::SessionConfig};

// Message types for WebSocket communication
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WSMessage {
    // Client -> Server messages
    Request {
        messages: Vec<GooseMessage>,
        session_working_dir: String,
        session_id: Option<String>,
    },
    
    ToolConfirmationResponse {
        request_id: String,
        confirmed: bool,
    },

    // Server -> Client messages
    Progress { 
        status: String, 
        message_count: usize 
    },
    Message { 
        role: String, 
        content: Vec<MessageContent> 
    },
    ToolConfirmation { 
        request_id: String, 
        tool: String, 
        args: Value 
    },
    Complete { 
        timing: TimingInfo 
    },
    Error { 
        message: String 
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimingInfo {
    total_duration_ms: u64,
    ai_calls: usize,
}

// WebSocket connection handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

// Main socket handling logic
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    
    // Create a broadcast channel for sending messages to the WebSocket
    let (tx, mut rx) = broadcast::channel::<WSMessage>(32);
    let tx_clone = tx.clone();

    // Spawn the message processor
    let process_handle = tokio::spawn(async move {
        // Wait for initial request from client
        while let Some(Ok(raw_message)) = ws_receiver.next().await {
            match raw_message {
                WSRawMessage::Text(text) => {
                    match serde_json::from_str::<WSMessage>(&text) {
                        Ok(WSMessage::Request { messages, session_working_dir, session_id }) => {
                            // Get a lock on the shared agent
                            let agent = state.agent.clone();
                            let agent = agent.read().await;
                            let agent = match agent.as_ref() {
                                Some(agent) => agent,
                                None => {
                                    let _ = tx_clone.send(WSMessage::Error {
                                        message: "No agent configured".to_string(),
                                    });
                                    continue;
                                }
                            };

                            // Get the provider
                            let provider = agent.provider().await;

                            // Generate session ID if not provided
                            let session_id = session_id.unwrap_or_else(session::generate_session_id);

                            // Create session config
                            let session_config = Some(SessionConfig {
                                id: session::Identifier::Name(session_id.clone()),
                                working_dir: PathBuf::from(session_working_dir.clone()),
                            });

                            // Get reply stream
                            let mut stream = match agent.reply(&messages, session_config).await {
                                Ok(stream) => stream,
                                Err(e) => {
                                    let _ = tx_clone.send(WSMessage::Error {
                                        message: e.to_string(),
                                    });
                                    continue;
                                }
                            };

                            // Track messages and timing
                            let start_time = std::time::Instant::now();
                            let mut ai_calls = 0;
                            let mut all_messages = messages.clone();

                            // Process stream
                            while let Some(msg) = stream.next().await {
                                match msg {
                                    Ok(message) => {
                                        // Track metrics
                                        match message.role {
                                            Role::Assistant => ai_calls += 1,
                                            _ => {}
                                        }

                                        // Store message
                                        all_messages.push(message.clone());

                                        // Send progress
                                        let _ = tx_clone.send(WSMessage::Progress {
                                            status: "processing".to_string(),
                                            message_count: ai_calls,
                                        });

                                        // Send message
                                        let ws_msg = WSMessage::Message {
                                            role: format!("{:?}", message.role),
                                            content: message.content,
                                        };
                                        let _ = tx_clone.send(ws_msg);

                                        // Store messages
                                        let session_path = session::get_path(session::Identifier::Name(session_id.clone()));
                                        let messages = all_messages.clone();
                                        let provider = provider.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = session::persist_messages(&session_path, &messages, Some(provider)).await {
                                                tracing::error!("Failed to store session history: {:?}", e);
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        let _ = tx_clone.send(WSMessage::Error {
                                            message: e.to_string(),
                                        });
                                        break;
                                    }
                                }
                            }

                            // Send completion
                            let _ = tx_clone.send(WSMessage::Complete {
                                timing: TimingInfo {
                                    total_duration_ms: start_time.elapsed().as_millis() as u64,
                                    ai_calls
                                }
                            });
                        }
                        Ok(WSMessage::ToolConfirmationResponse { request_id, confirmed }) => {
                            if let Some(agent) = state.agent.read().await.as_ref() {
                                agent.handle_confirmation(request_id, confirmed).await;
                            }
                        }
                        Ok(WSMessage::Progress { .. }) |
                        Ok(WSMessage::Message { .. }) |
                        Ok(WSMessage::ToolConfirmation { .. }) |
                        Ok(WSMessage::Complete { .. }) |
                        Ok(WSMessage::Error { .. }) => {
                            let _ = tx_clone.send(WSMessage::Error {
                                message: "Received server-only message type from client".to_string()
                            });
                        }
                        Err(e) => {
                            let _ = tx_clone.send(WSMessage::Error {
                                message: format!("Invalid message format: {}", e),
                            });
                        }
                    }
                }
                WSRawMessage::Close(_) => break,
                _ => {} // Ignore other message types
            }
        }
    });

    // Forward messages from the broadcast channel to the WebSocket
    while let Ok(msg) = rx.recv().await {
        if let Err(e) = ws_sender.send(WSRawMessage::Text(serde_json::to_string(&msg).unwrap().into())).await {
            eprintln!("Failed to send WebSocket message: {}", e);
            break;
        }
    }

    // Clean up
    let _ = process_handle.await;
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use goose::agents::AgentFactory;
    use goose::providers::base::{Provider, ProviderUsage, Usage, ProviderMetadata};
    use goose::model::ModelConfig;
    use mcp_core::tool::Tool;
    use anyhow::Result;
    use goose::providers::errors::ProviderError;

    // Mock provider for testing
    #[derive(Clone)]
    struct MockProvider {
        responses: Arc<RwLock<Vec<GooseMessage>>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                responses: Arc::new(RwLock::new(vec![
                    GooseMessage::assistant().with_text("Hello!"),
                ])),
            }
        }
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            ModelConfig::new("test-model".to_string())
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[GooseMessage],
            _tools: &[Tool],
        ) -> Result<(GooseMessage, ProviderUsage), ProviderError> {
            let responses = self.responses.read().await;
            Ok((
                responses[0].clone(),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    // Helper function to create a test server
    async fn create_test_server() -> (SocketAddr, AppState) {
        // Create mock agent
        let mock_provider = Box::new(MockProvider::new());
        let agent = AgentFactory::create("truncate", mock_provider).unwrap();
        
        // Create app state
        let state = AppState {
            config: Arc::new(Mutex::new(HashMap::new())),
            agent: Arc::new(RwLock::new(Some(agent))),
            secret_key: "test-secret".to_string(),
        };

        // Create router
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(state.clone());

        // Start server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (addr, state)
    }

    #[tokio::test]
    async fn test_websocket_chat_request() {
        // Create test server
        let (addr, _state) = create_test_server().await;

        // Connect to WebSocket
        let url = format!("ws://{}/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Create a chat request
        let request = WSMessage::Request {
            messages: vec![GooseMessage::user().with_text("Hello")],
            session_working_dir: "/tmp".to_string(),
            session_id: Some("test-session".to_string()),
        };

        // Send request
        let msg = tungstenite::Message::Text(serde_json::to_string(&request).unwrap().into());
        write.send(msg).await.unwrap();

        // Collect responses
        let mut received_progress = false;
        let mut received_message = false;
        let mut received_complete = false;

        while let Some(Ok(msg)) = read.next().await {
            match msg {
                tungstenite::Message::Text(text) => {
                    let response: WSMessage = serde_json::from_str(&text).unwrap();
                    match response {
                        WSMessage::Progress { .. } => {
                            received_progress = true;
                        }
                        WSMessage::Message { .. } => {
                            received_message = true;
                        }
                        WSMessage::Complete { .. } => {
                            received_complete = true;
                            break;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        assert!(received_progress, "Should receive progress update");
        assert!(received_message, "Should receive chat message");
        assert!(received_complete, "Should receive completion");
    }

    #[tokio::test]
    async fn test_websocket_tool_confirmation() {
        // Create test server
        let (addr, _state) = create_test_server().await;

        // Connect to WebSocket
        let url = format!("ws://{}/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Create a tool confirmation response
        let confirmation = WSMessage::ToolConfirmationResponse {
            request_id: "test-123".to_string(),
            confirmed: true,
        };

        // Send confirmation
        let msg = tungstenite::Message::Text(serde_json::to_string(&confirmation).unwrap().into());
        write.send(msg).await.unwrap();

        // Should not receive any response for tool confirmations
        let timeout = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            read.next()
        ).await;

        assert!(timeout.is_err(), "Should not receive response for tool confirmation");
    }
}