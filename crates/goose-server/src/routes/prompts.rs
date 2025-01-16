use crate::state::AppState;
use axum::{
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::Serialize;

#[derive(Serialize)]
struct ListPromptsResponse {
    prompts: Vec<String>,
}

#[derive(Serialize)]
struct GetPromptResponse {
    name: String,
    content: String,
}

async fn list_prompts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListPromptsResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agent = state.agent.lock().await;
    let agent = agent.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    
    // Get prompts through agent passthrough
    let result = agent
        .passthrough("prompts", serde_json::json!({ "method": "list" }))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let prompts = result
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    Ok(Json(ListPromptsResponse { prompts }))
}

async fn get_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(prompt_name): Path<String>,
) -> Result<Json<GetPromptResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agent = state.agent.lock().await;
    let agent = agent.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    
    // Get prompt through agent passthrough
    let result = agent
        .passthrough(
            "prompts",
            serde_json::json!({
                "method": "get",
                "name": prompt_name
            })
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let content = result
        .as_str()
        .map(String::from)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GetPromptResponse {
        name: prompt_name,
        content,
    }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/prompts/list", post(list_prompts))
        .route("/prompts/get/:prompt_name", post(get_prompt))
        .with_state(state)
}
