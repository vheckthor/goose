use axum::{
    extract::State,
    routing::{post},
    Json, Router,
    http::{HeaderMap, StatusCode},
};
use std::collections::HashMap;
use goose::agents::extension::Envs;
use goose::config::ExtensionConfig;
use goose::gooselings::Gooseling;
use goose::message::Message;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateGooselingRequest {
    messages: Vec<Message>,
    // Required metadata
    title: String,
    description: String,
    // Optional fields
    #[serde(default)]
    activities: Option<Vec<String>>,
    #[serde(default)]
    author: Option<AuthorRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorRequest {
    #[serde(default)]
    contact: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateGooselingResponse {
    gooseling: Gooseling,
}

/// Create a Gooseling configuration from the current state of an agent
async fn create_gooseling(
    State(state): State<AppState>,
    Json(request): Json<CreateGooselingRequest>,
) -> Result<Json<CreateGooselingResponse>, StatusCode> {
    let agent = state.agent.read().await;
    let agent = agent.as_ref().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    
    // Create base gooseling from agent state and messages
    let mut gooseling = agent.create_gooseling(request.messages).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update with user-provided metadata
    gooseling.title = request.title;
    gooseling.description = request.description;
    if request.activities.is_some(){
        gooseling.activities = request.activities
    };
    
    // Add author if provided
    if let Some(author_req) = request.author {
        gooseling.author = Some(goose::gooselings::Author {
            contact: author_req.contact,
            metadata: author_req.metadata,
        });
    }

    Ok(Json(CreateGooselingResponse { gooseling }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/gooseling/create", post(create_gooseling))
        .with_state(state)
}