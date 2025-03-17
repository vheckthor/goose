use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use goose::{
    agents::SessionConfig,
    message::{Message, MessageContent},
};

#[derive(Debug, Deserialize)]
pub struct CompletionRequest {
    messages: Vec<Message>,
    model: Option<String>,
    stream: Option<bool>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct CompletionResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    response: String,
    usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

pub async fn completions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CompletionRequest>,
) -> Result<Json<CompletionResponse>, StatusCode> {
    // Verify API key if present
    if let Some(api_key) = headers.get("X-API-Key") {
        if api_key.to_str().unwrap_or("") != state.secret_key {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agent = state.agent.clone();
    let agent = agent.read().await;
    let agent = agent.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get the provider to know which model we're using
    let provider = agent.provider().await;
    let model = request.model.unwrap_or_else(|| provider.to_string());

    // Create messages for the conversation
    let messages = request.messages;

    // Get response from agent
    let mut response_text = String::new();
    let mut stream = match agent
        .reply(
            &messages,
            Some(SessionConfig {
                id: goose::session::Identifier::Random,
                working_dir: std::env::current_dir().unwrap(),
            }),
        )
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to start reply stream: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    use futures::StreamExt;
    while let Some(response) = stream.next().await {
        match response {
            Ok(message) => {
                for content in &message.content {
                    if let MessageContent::Text(text) = content {
                        response_text.push_str(&text.text);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error processing message: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Create response
    Ok(Json(CompletionResponse {
        id: uuid::Uuid::new_v4().to_string(),
        object: "completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model,
        response: response_text.trim().to_string(),
        usage: Usage {
            prompt_tokens: 0,  // TODO: Implement token counting
            completion_tokens: 0,
            total_tokens: 0,
        },
    }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/completions", post(completions_handler))
        .with_state(state)
}