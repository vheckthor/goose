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
    #[serde(rename = "tool_call")]
    ToolCall {
        tool_name: String,
        arguments: serde_json::Value,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

pub async fn handle_web(port: u16, host: String, open: bool) -> Result<()> {
    // Setup logging
    crate::logging::setup_logging(Some("goose-web"), None)?;

    // Initialize agent
    let agent = Agent::new();
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

    println!("\n🪿 Starting Goose web server on http://{}", addr);
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
    use goose::agents::SessionConfig;
    use goose::session;
    use futures::StreamExt;
    use std::path::PathBuf;
    
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
        working_dir: PathBuf::from("."),
        schedule_id: None,
    };
    
    // Get response from agent
    let mut accumulated_response = String::new();
    match agent.reply(&messages, Some(session_config)).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => {
                        // Extract text content from the message and send it
                        for content in &message.content {
                            if let goose::message::MessageContent::Text(text) = content {
                                accumulated_response.push_str(&text.text);
                                
                                // Send the partial response
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
