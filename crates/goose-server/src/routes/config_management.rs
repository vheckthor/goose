use axum::{routing::{get, post, delete}, Json, http::StatusCode, Router, extract::{Query, State}};
use serde::{Deserialize, Serialize};
use serde_yaml::{Value, to_string as to_yaml_string};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use crate::state::AppState;
use std::fs;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct UpsertConfigQuery {
    /// The configuration key to upsert
    pub key: String,
    /// The value to set for the configuration
    pub value: Value,
    /// Whether this configuration value should be treated as a secret
    #[allow(dead_code)]  // Used in OpenAPI schema for documentation
    pub is_secret: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct ConfigKeyQuery {
    /// The configuration key to operate on
    pub key: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtensionQuery {
    /// The name of the extension
    pub name: String,
    /// The configuration for the extension
    pub config: Value,
}

#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    /// The configuration values
    pub config: HashMap<String, Value>,
}

const CONFIG_FILE_PATH: &str = "~/.config/goose/config.yaml";

/// Upsert a configuration value
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
    Query(query): Query<UpsertConfigQuery>,
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Result<Json<Value>, StatusCode> {
    let mut config = state.lock().await;
    let key = query.key;
    config.insert(key.clone(), query.value.clone());
    persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Value::String(format!("Upserted key {}", key))))
}

/// Remove a configuration value
#[utoipa::path(
    delete,
    path = "/config/remove",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value removed successfully", body = String),
        (status = 404, description = "Configuration key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_config(
    Query(query): Query<ConfigKeyQuery>,
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Result<Json<String>, StatusCode> {
    let mut config = state.lock().await;
    let key = query.key;
    if config.remove(&key).is_some() {
        persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(format!("Removed key {}", key)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Read a configuration value
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
    Query(query): Query<ConfigKeyQuery>,
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Result<Json<Value>, StatusCode> {
    let config = state.lock().await;
    if let Some(value) = config.get(&query.key) {
        Ok(Json(value.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Add an extension configuration
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
            persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(format!("Added extension {}", extension.name)));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

/// Remove an extension configuration
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
    Query(query): Query<ConfigKeyQuery>,
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Result<Json<String>, StatusCode> {
    let mut config = state.lock().await;
    if let Some(extensions) = config.get_mut("extensions") {
        if let Value::Mapping(map) = extensions {
            if map.remove(&Value::String(query.key.clone())).is_some() {
                persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Json(format!("Removed extension {}", query.key)));
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

/// Read all configuration values
#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = ConfigResponse)
    )
)]
pub async fn read_all_config(
    State(state): State<Arc<Mutex<HashMap<String, Value>>>>
) -> Json<HashMap<String, Value>> {
    let config = state.lock().await;
    Json(config.clone())
}

fn persist_config(config: &HashMap<String, Value>) -> Result<(), StatusCode> {
    to_yaml_string(config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .and_then(|yaml_string| {
            fs::write(CONFIG_FILE_PATH, yaml_string)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", delete(remove_config))
        .route("/config/read", get(read_config))
        .route("/config/extension", post(add_extension))
        .route("/config/extension", delete(remove_extension))
        .with_state(state.config)
}