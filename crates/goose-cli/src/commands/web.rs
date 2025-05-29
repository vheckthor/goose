use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use goose::agents::Agent;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::error;

// Simple message structure for the web interface
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: i64,
}

type SessionStore = Arc<RwLock<std::collections::HashMap<String, Arc<Mutex<Vec<ChatMessage>>>>>>;

#[derive(Clone)]
struct AppState {
    agent: Arc<Agent>,
    sessions: SessionStore,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum WebSocketMessage {
    #[serde(rename = "message")]
    Message {
        content: String,
        session_id: String,
        timestamp: i64,
    },
    #[serde(rename = "response")]
    Response {
        content: String,
        role: String,
        timestamp: i64,
    },
    #[serde(rename = "tool_request")]
    ToolRequest {
        id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    #[serde(rename = "tool_response")]
    ToolResponse {
        id: String,
        result: serde_json::Value,
        is_error: bool,
    },
    #[serde(rename = "tool_confirmation")]
    ToolConfirmation {
        id: String,
        tool_name: String,
        arguments: serde_json::Value,
        needs_confirmation: bool,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "thinking")]
    Thinking { message: String },
    #[serde(rename = "context_exceeded")]
    ContextExceeded { message: String },
}

pub async fn handle_web(port: u16, host: String, open: bool) -> Result<()> {
    // Setup logging
    crate::logging::setup_logging(Some("goose-web"), None)?;
    
    // Load config and create agent just like the CLI does
    let config = goose::config::Config::global();
    
    let provider_name: String = match config.get_param("GOOSE_PROVIDER") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("No provider configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };

    let model: String = match config.get_param("GOOSE_MODEL") {
        Ok(m) => m,
        Err(_) => {
            eprintln!("No model configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };
    
    let model_config = goose::model::ModelConfig::new(model.clone());
    
    // Create the agent
    let agent = Agent::new();
    let provider = goose::providers::create(&provider_name, model_config)?;
    agent.update_provider(provider).await?;
    
    // Load and enable extensions from config
    let extensions = goose::config::ExtensionConfigManager::get_all()?;
    for ext_config in extensions {
        if ext_config.enabled {
            if let Err(e) = agent.add_extension(ext_config.config.clone()).await {
                eprintln!("Warning: Failed to load extension {}: {}", ext_config.config.name(), e);
            }
        }
    }
    
    let state = AppState {
        agent: Arc::new(agent),
        sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
    };

    // Build router
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(websocket_handler))
        .route("/api/health", get(health_check))
        .route("/static/{*path}", get(serve_static))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    
    println!("\n🪿 Starting Goose web server");
    println!("   Provider: {} | Model: {}", provider_name, model);
    println!("   Working directory: {}", std::env::current_dir()?.display());
    println!("   Server: http://{}", addr);
    println!("   Press Ctrl+C to stop\n");
    
    if open {
        // Open browser
        let url = format!("http://{}", addr);
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("Failed to open browser: {}", e);
        }
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match path.as_str() {
        "style.css" => (
            [("content-type", "text/css")],
            include_str!("../../static/style.css"),
        )
            .into_response(),
        "script.js" => (
            [("content-type", "application/javascript")],
            include_str!("../../static/script.js"),
        )
            .into_response(),
        _ => (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "goose-web"
    }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Send initial connection confirmation
    {
        let mut sender = sender.lock().await;
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&WebSocketMessage::Response {
                    content: "Connected to Goose! How can I help you today?".to_string(),
                    role: "assistant".to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                })
                .unwrap()
                .into(),
            ))
            .await;
    }

    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(WebSocketMessage::Message {
                        content,
                        session_id,
                        ..
                    }) = serde_json::from_str::<WebSocketMessage>(&text.to_string())
                    {
                        // Get or create session
                        let session = {
                            let sessions = state.sessions.read().await;
                            if let Some(session) = sessions.get(&session_id) {
                                session.clone()
                            } else {
                                drop(sessions);
                                let mut sessions = state.sessions.write().await;
                                let new_session = Arc::new(Mutex::new(Vec::new()));
                                sessions.insert(session_id.clone(), new_session.clone());
                                new_session
                            }
                        };

                        // Clone sender for async processing
                        let sender_clone = sender.clone();
                        let agent = state.agent.clone();
                        
                        // Process message in a separate task to allow streaming
                        tokio::spawn(async move {
                            if let Err(e) = process_message_streaming(&agent, session, content, sender_clone).await {
                                error!("Error processing message: {}", e);
                            }
                        });
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

async fn process_message_streaming(
    agent: &Agent,
    session: Arc<Mutex<Vec<ChatMessage>>>,
    content: String,
    sender: Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
) -> Result<()> {
    use goose::message::Message as GooseMessage;
    use goose::message::MessageContent;
    use goose::agents::SessionConfig;
    use goose::session;
    use futures::StreamExt;
    
    
    // Create a user message
    let user_message = GooseMessage::user().with_text(content.clone());
    
    // Get existing messages from session
    let mut messages = {
        let session_messages = session.lock().await;
        session_messages.iter().map(|cm| {
            if cm.role == "user" {
                GooseMessage::user().with_text(cm.content.clone())
            } else {
                GooseMessage::assistant().with_text(cm.content.clone())
            }
        }).collect::<Vec<_>>()
    };
    
    // Add the new user message
    messages.push(user_message);
    
    // Store the user message in our session
    {
        let mut session_messages = session.lock().await;
        session_messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }
    
    // Check if provider is configured
    let provider = agent.provider().await;
    if provider.is_err() {
        let error_msg = "I'm not properly configured yet. Please configure a provider through the CLI first using `goose configure`.".to_string();
        let mut sender = sender.lock().await;
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&WebSocketMessage::Response {
                    content: error_msg,
                    role: "assistant".to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                })
                .unwrap()
                .into(),
            ))
            .await;
        return Ok(());
    }
    
    // Create a session config
    let session_config = SessionConfig {
        id: session::Identifier::Name("web-session".to_string()),
        working_dir: std::env::current_dir()?,
        schedule_id: None,
    };
    
    // Get response from agent
    let mut accumulated_response = String::new();
    match agent.reply(&messages, Some(session_config)).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => {
                        // Handle different message content types
                        for content in &message.content {
                            match content {
                                MessageContent::Text(text) => {
                                    accumulated_response.push_str(&text.text);
                                    
                                    // Send the text response
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::Response {
                                                content: text.text.clone(),
                                                role: "assistant".to_string(),
                                                timestamp: chrono::Utc::now().timestamp_millis(),
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                }
                                MessageContent::ToolRequest(req) => {
                                    // Send tool request notification
                                    let mut sender = sender.lock().await;
                                    if let Ok(tool_call) = &req.tool_call {
                                        let _ = sender
                                            .send(Message::Text(
                                                serde_json::to_string(&WebSocketMessage::ToolRequest {
                                                    id: req.id.clone(),
                                                    tool_name: tool_call.name.clone(),
                                                    arguments: tool_call.arguments.clone(),
                                                })
                                                .unwrap()
                                                .into(),
                                            ))
                                            .await;
                                    }
                                }
                                MessageContent::ToolResponse(resp) => {
                                    // Send tool response
                                    let mut sender = sender.lock().await;
                                    let (result, is_error) = match &resp.tool_result {
                                        Ok(contents) => {
                                            // Convert contents to JSON
                                            let json_contents: Vec<serde_json::Value> = contents.iter().map(|c| {
                                                match c {
                                                    mcp_core::content::Content::Text(text) => {
                                                        serde_json::json!({
                                                            "type": "text",
                                                            "text": text.text
                                                        })
                                                    }
                                                    mcp_core::content::Content::Image(image) => {
                                                        serde_json::json!({
                                                            "type": "image",
                                                            "data": image.data,
                                                            "mimeType": image.mime_type
                                                        })
                                                    }
                                                    mcp_core::content::Content::Resource(resource) => {
                                                        match &resource.resource {
                                                            mcp_core::resource::ResourceContents::TextResourceContents { uri, mime_type, text } => {
                                                                serde_json::json!({
                                                                    "type": "resource",
                                                                    "uri": uri,
                                                                    "mimeType": mime_type.as_deref().unwrap_or("text/plain"),
                                                                    "text": text
                                                                })
                                                            }
                                                            mcp_core::resource::ResourceContents::BlobResourceContents { uri, mime_type, blob } => {
                                                                serde_json::json!({
                                                                    "type": "resource",
                                                                    "uri": uri,
                                                                    "mimeType": mime_type.as_deref().unwrap_or("application/octet-stream"),
                                                                    "blob": blob
                                                                })
                                                            }
                                                        }
                                                    }
                                                }
                                            }).collect();
                                            (serde_json::json!(json_contents), false)
                                        }
                                        Err(e) => (serde_json::json!({"error": e.to_string()}), true)
                                    };
                                    
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::ToolResponse {
                                                id: resp.id.clone(),
                                                result,
                                                is_error,
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                }
                                MessageContent::ToolConfirmationRequest(confirmation) => {
                                    // Send tool confirmation request
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::ToolConfirmation {
                                                id: confirmation.id.clone(),
                                                tool_name: confirmation.tool_name.clone(),
                                                arguments: confirmation.arguments.clone(),
                                                needs_confirmation: true,
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                    
                                    // For now, auto-approve in web mode
                                    // TODO: Implement proper confirmation UI
                                    agent.handle_confirmation(
                                        confirmation.id.clone(),
                                        goose::permission::PermissionConfirmation {
                                            principal_type: goose::permission::permission_confirmation::PrincipalType::Tool,
                                            permission: goose::permission::Permission::AllowOnce,
                                        }
                                    ).await;
                                }
                                MessageContent::Thinking(thinking) => {
                                    // Send thinking indicator
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::Thinking {
                                                message: thinking.thinking.clone(),
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                }
                                MessageContent::ContextLengthExceeded(msg) => {
                                    // Send context exceeded notification
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::ContextExceeded {
                                                message: msg.msg.clone(),
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                    
                                    // For now, auto-summarize in web mode
                                    // TODO: Implement proper UI for context handling
                                    let (summarized_messages, _) = agent.summarize_context(&messages).await?;
                                    messages = summarized_messages;
                                }
                                _ => {
                                    // Handle other message types as needed
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in message stream: {}", e);
                        let mut sender = sender.lock().await;
                        let _ = sender
                            .send(Message::Text(
                                serde_json::to_string(&WebSocketMessage::Error {
                                    message: format!("Error: {}", e),
                                })
                                .unwrap()
                                .into(),
                            ))
                            .await;
                        break;
                    }
                }
            }
        }
        Err(e) => {
            error!("Error calling agent: {}", e);
            let mut sender = sender.lock().await;
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&WebSocketMessage::Error {
                        message: format!("Error: {}", e),
                    })
                    .unwrap()
                    .into(),
                ))
                .await;
        }
    }
    
    // Store the complete assistant response in our session
    if !accumulated_response.is_empty() {
        let mut session_messages = session.lock().await;
        session_messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: accumulated_response,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }
    
    Ok(())
}

// Add webbrowser dependency for opening browser
use webbrowser;
