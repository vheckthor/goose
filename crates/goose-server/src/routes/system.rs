use std::collections::HashMap;

use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::{
    agents::{system::Envs, SystemConfig},
    key_manager::{get_keyring_secret, KeyRetrievalStrategy},
};
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

/// Enum representing the different types of system configuration requests.
#[derive(Deserialize)]
#[serde(tag = "type")]
enum SystemConfigRequest {
    /// Server-Sent Events (SSE) system.
    #[serde(rename = "sse")]
    Sse {
        /// The URI endpoint for the SSE system.
        uri: String,
        /// List of environment variable keys. The server will fetch their values from the keyring.
        env_keys: Vec<String>,
    },
    /// Standard I/O (stdio) system.
    #[serde(rename = "stdio")]
    Stdio {
        /// The command to execute.
        cmd: String,
        /// Arguments for the command.
        args: Vec<String>,
        /// List of environment variable keys. The server will fetch their values from the keyring.
        env_keys: Vec<String>,
    },
    /// Built-in system that is part of the goose binary.
    #[serde(rename = "builtin")]
    Builtin {
        /// The name of the built-in system.
        name: String,
    },
}

/// Response structure for adding a system.
///
/// - `error`: Indicates whether an error occurred (`true`) or not (`false`).
/// - `message`: Provides detailed error information when `error` is `true`.
#[derive(Serialize)]
struct SystemResponse {
    error: bool,
    message: Option<String>,
}

/// Handler for adding a new system configuration.
async fn add_system(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SystemConfigRequest>,
) -> Result<Json<SystemResponse>, StatusCode> {
    // Verify the presence and validity of the secret key.
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Initialize a vector to collect any missing keys.
    let mut missing_keys = Vec::new();

    // Construct SystemConfig with Envs populated from keyring based on provided env_keys.
    let system_config: SystemConfig = match request {
        SystemConfigRequest::Sse { uri, env_keys } => {
            let mut env_map = HashMap::new();
            for key in env_keys {
                match get_keyring_secret(&key, KeyRetrievalStrategy::KeyringOnly) {
                    Ok(value) => {
                        env_map.insert(key, value);
                    }
                    Err(_) => {
                        missing_keys.push(key);
                    }
                }
            }

            if !missing_keys.is_empty() {
                return Ok(Json(SystemResponse {
                    error: true,
                    message: Some(format!(
                        "Missing secrets for keys: {}",
                        missing_keys.join(", ")
                    )),
                }));
            }

            SystemConfig::Sse {
                uri,
                envs: Envs::new(env_map),
            }
        }
        SystemConfigRequest::Stdio {
            cmd,
            args,
            env_keys,
        } => {
            let mut env_map = HashMap::new();
            for key in env_keys {
                match get_keyring_secret(&key, KeyRetrievalStrategy::KeyringOnly) {
                    Ok(value) => {
                        env_map.insert(key, value);
                    }
                    Err(_) => {
                        missing_keys.push(key);
                    }
                }
            }

            if !missing_keys.is_empty() {
                return Ok(Json(SystemResponse {
                    error: true,
                    message: Some(format!(
                        "Missing secrets for keys: {}",
                        missing_keys.join(", ")
                    )),
                }));
            }

            SystemConfig::Stdio {
                cmd,
                args,
                envs: Envs::new(env_map),
            }
        }
        SystemConfigRequest::Builtin { name } => SystemConfig::Builtin { name },
    };

    // Acquire a lock on the agent and attempt to add the system.
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    let response = agent.add_system(system_config).await;

    // Respond with the result.
    match response {
        Ok(_) => Ok(Json(SystemResponse {
            error: false,
            message: None,
        })),
        Err(e) => {
            eprintln!("Failed to add system configuration: {:?}", e);
            Ok(Json(SystemResponse {
                error: true,
                message: Some("Failed to add system configuration".to_string()),
            }))
        }
    }
}

/// Registers the `/systems/add` route with the Axum router.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/systems/add", post(add_system))
        .with_state(state)
}
