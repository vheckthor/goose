use anyhow::Result;
use goose::agents::Agent;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<Option<Agent>>,
    pub secret_key: String,
    pub config: Arc<Mutex<HashMap<String, Value>>>,
}

impl AppState {
    pub async fn new(secret_key: String) -> Result<Self> {
        Ok(Self {
            agent: Arc::new(None),
            secret_key,
            config: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn with_agent(self, agent: Agent) -> Self {
        Self {
            agent: Arc::new(Some(agent)),
            secret_key: self.secret_key,
            config: self.config,
        }
    }
}
