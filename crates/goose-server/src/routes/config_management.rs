use axum::{Json, http::StatusCode};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use serde_yaml::{Value, to_string as to_yaml_string};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use crate::state::AppState;
use std::fs;

#[derive(Deserialize)]
struct UpsertConfigQuery {
    key: String,
    value: Value,
    is_secret: Option<bool>,
}

#[derive(Deserialize)]
struct ConfigKeyQuery {
    key: String,
}

#[derive(Deserialize)]
struct ExtensionQuery {
    name: String,
    config: Value,
}

// File path to the config, could be an environment variable or similar in reality
const CONFIG_FILE_PATH: &str = "~/.config/goose/config.yaml";

// Handler code for upserting a config value
async fn upsert_config(
    Query(query): Query<UpsertConfigQuery>,
    state: Arc<Mutex<HashMap<String, Value>>>
) -> Result<Json<Value>, StatusCode> {
    let mut config = state.lock().await;
    let key = query.key;
    config.insert(key.clone(), query.value.clone());
    persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Value::String(format!("Upserted key {}", key))))
}

// Handler code for removing a config value
async fn remove_config(
    Query(query): Query<ConfigKeyQuery>,
    state: Arc<Mutex<HashMap<String, Value>>>
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

// Handler code for reading a config value
async fn read_config(
    Query(query): Query<ConfigKeyQuery>,
    state: Arc<Mutex<HashMap<String, Value>>>
) -> Result<Json<Value>, StatusCode> {
    let config = state.lock().await;
    if let Some(value) = config.get(&query.key) {
        Ok(Json(value.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Handler code for adding an extension
async fn add_extension(
    Json(extension): Json<ExtensionQuery>,
    state: Arc<Mutex<HashMap<String, Value>>>
) -> Result<Json<String>, StatusCode> {
    let mut config = state.lock().await;
    if let Some(extensions) = config.get_mut("extensions") {
        if let Value::Mapping(map) = extensions {
            // Assume extension is added
            map.insert(Value::String(extension.name.clone()), extension.config);
            persist_config(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(format!("Added extension {}", extension.name)));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

// Handler code for removing an extension
async fn remove_extension(
    Query(query): Query<ConfigKeyQuery>,
    state: Arc<Mutex<HashMap<String, Value>>>
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

// Handler code for reading all config values
async fn read_all_config(
    state: Arc<Mutex<HashMap<String, Value>>>
) -> Json<HashMap<String, Value>> {
    let config = state.lock().await;
    Json(config.clone())
}

// Persists the current state of the config to a YAML file
fn persist_config(config: &HashMap<String, Value>) -> Result<(), std::io::Error> {
    let yaml_string = to_yaml_string(config)?;
    fs::write(CONFIG_FILE_PATH, yaml_string)?;
    Ok(())
}
