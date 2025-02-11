use axum::{routing::{get, post, delete}, Json, http::StatusCode, Router, extract::{Query, State}};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use crate::{state::AppState, config_manager::ConfigManager};
use utoipa::ToSchema;
use tracing;

fn config_manager() -> ConfigManager {
    let config_dir = dirs::home_dir()
        .expect("goose requires a home dir")
        .join(".config")
        .join("goose");
    ConfigManager::new("goose", config_dir.to_str().unwrap())
}

#[derive(Deserialize, ToSchema)]
pub struct UpsertConfigQuery {
    pub key: String,
    pub value: Value,
    #[allow(dead_code)]
    pub is_secret: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct ConfigKeyQuery {
    pub key: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtensionQuery {
    pub name: String,
    pub config: Value,
}

#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    pub config: HashMap<String, Value>,
}

#[utoipa::path(
    post,
    path = "/config/upsert",
    request_body = UpsertConfigQuery,
    responses(
        (status = 200, description = "Configuration value upserted successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upsert_config(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<UpsertConfigQuery>
) -> Result<Json<Value>, StatusCode> {
    let mut config = state.lock().await;

    // Use ConfigManager to persist config
    config_manager().set(&query.key, query.value.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    config.insert(query.key.clone(), query.value);
    Ok(Json(Value::String(format!("Upserted key {}", query.key))))
}

#[utoipa::path(
    post,
    path = "/config/remove",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value removed successfully", body = String),
        (status = 404, description = "Configuration key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_config(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>
) -> Result<Json<String>, StatusCode> {
    if config_manager().delete(&query.key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        let mut config = state.lock().await;
        config.remove(&query.key);
        return Ok(Json(format!("Removed key {}", query.key)));
    }
    Err(StatusCode::NOT_FOUND)
}

#[utoipa::path(
    get,
    path = "/config/read",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value retrieved successfully", body = Value),
        (status = 404, description = "Configuration key not found")
    )
)]
pub async fn read_config(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>
) -> Result<Json<Value>, StatusCode> {
    if let Ok(value) = config_manager().get(&query.key) {
        return Ok(Json(value))
    }
    Err(StatusCode::NOT_FOUND)
}

#[utoipa::path(
    post,
    path = "/config/extension",
    request_body = ExtensionQuery,
    responses(
        (status = 200, description = "Extension added successfully", body = String),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn add_extension(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(extension): Json<ExtensionQuery>
) -> Result<Json<String>, StatusCode> {
    let mut config = state.lock().await;
    if let Some(extensions) = config.get_mut("extensions") {
        if let Value::Mapping(map) = extensions {
            map.insert(Value::String(extension.name.clone()), extension.config);
            config_manager().set(&format!("extensions.{}", extension.name), map.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(format!("Added extension {}", extension.name)));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

#[utoipa::path(
    delete,
    path = "/config/extension",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Extension removed successfully", body = String),
        (status = 404, description = "Extension not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_extension(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>
) -> Result<Json<String>, StatusCode> {
    let mut config = state.lock().await;
    if let Some(extensions) = config.get_mut("extensions") {
        if let Value::Mapping(map) = extensions {
            if map.remove(&Value::String(query.key.clone())).is_some() {
                config_manager().delete(&format!("extensions.{}", query.key)).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Json(format!("Removed extension {}", query.key)));
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = ConfigResponse)
    )
)]
pub async fn read_all_config(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Json<ConfigResponse> {
    let config = config_manager().get_all().unwrap_or_default();
    Json(ConfigResponse { config })
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/extension", post(add_extension))
        .route("/config/extension", delete(remove_extension))
        .with_state(state.config)
}
