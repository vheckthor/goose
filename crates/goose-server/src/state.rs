use goose::agents::Agent;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<Mutex<Option<Arc<Agent>>>>,
    pub secret_key: String,
    pub config: Arc<Mutex<HashMap<String, Value>>>,
}

impl AppState {
    pub async fn new(secret_key: String) -> Self {
        Self {
            agent: Arc::new(Mutex::new(None)),
            secret_key,
            config: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn set_agent(&self, agent: Agent) {
        let mut agent_guard = self.agent.lock().await;
        *agent_guard = Some(Arc::new(agent));
    }

    pub async fn get_agent(&self) -> Result<Arc<Agent>, anyhow::Error> {
        let agent_guard = self.agent.lock().await;
        agent_guard
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Agent needs to be created first."))
    }
}
