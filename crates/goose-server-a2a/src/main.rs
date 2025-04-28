use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use anyhow::Result;

// A2A Protocol Types
#[derive(Debug, Serialize, Deserialize)]
struct JSONRPCRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JSONRPCError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JSONRPCResponse<T> {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    result: Option<T>,
    error: Option<JSONRPCError>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    #[serde(rename = "type")]
    part_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct TaskStatus {
    state: String,
    message: Option<Message>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct Task {
    id: String,
    session_id: Option<String>,
    status: TaskStatus,
    artifacts: Option<Vec<serde_json::Value>>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct TaskStatusUpdateEvent {
    id: String,
    status: TaskStatus,
    final_: bool,
}

// Application state
#[derive(Clone)]
struct AppState {
    // Add Goose agent and other state here
}

impl AppState {
    fn new() -> Self {
        Self {}
    }
}

// Error handling
impl<T> JSONRPCResponse<T> {
    fn error(code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(JSONRPCError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    fn success(id: Option<serde_json::Value>, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
}

// Route handlers
async fn handle_rpc_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<JSONRPCRequest>,
) -> Result<Json<JSONRPCResponse<serde_json::Value>>, StatusCode> {
    match request.method.as_str() {
        "tasks/send" => {
            // Handle regular task send
            let task = Task {
                id: "task-123".to_string(), // Generate real ID
                session_id: None,
                status: TaskStatus {
                    state: "working".to_string(),
                    message: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
                artifacts: None,
                metadata: None,
            };
            
            Ok(Json(JSONRPCResponse::success(request.id, serde_json::to_value(task).unwrap())))
        }
        "tasks/sendSubscribe" => {
            // This should be handled by the streaming endpoint
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
        "tasks/get" => {
            // Handle task status retrieval
            let task = Task {
                id: "task-123".to_string(),
                session_id: None,
                status: TaskStatus {
                    state: "working".to_string(),
                    message: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
                artifacts: None,
                metadata: None,
            };
            
            Ok(Json(JSONRPCResponse::success(request.id, serde_json::to_value(task).unwrap())))
        }
        "tasks/cancel" => {
            // Handle task cancellation
            let task = Task {
                id: "task-123".to_string(),
                session_id: None,
                status: TaskStatus {
                    state: "canceled".to_string(),
                    message: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
                artifacts: None,
                metadata: None,
            };
            
            Ok(Json(JSONRPCResponse::success(request.id, serde_json::to_value(task).unwrap())))
        }
        _ => Ok(Json(JSONRPCResponse::error(-32601, "Method not found")))
    }
}

type SSEStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

async fn handle_streaming_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<JSONRPCRequest>,
) -> Sse<SSEStream> {
    let stream = stream::once(async move {
        let event = TaskStatusUpdateEvent {
            id: "task-123".to_string(),
            status: TaskStatus {
                state: "working".to_string(),
                message: Some(Message {
                    role: "agent".to_string(),
                    parts: vec![Part {
                        part_type: "text".to_string(),
                        text: "Working on it...".to_string(),
                    }],
                }),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
            final_: false,
        };

        let response = JSONRPCResponse::success(request.id, serde_json::to_value(event).unwrap());
        let data = serde_json::to_string(&response).unwrap();
        Ok(Event::default().data(data))
    });

    Sse::new(Box::pin(stream) as SSEStream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize application state
    let state = Arc::new(AppState::new());

    // Build our application with routes
    let app = Router::new()
        .route("/", post(handle_rpc_request))
        .route("/stream", post(handle_streaming_request))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Run it
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 41241));
    info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}