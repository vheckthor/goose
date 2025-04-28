use async_trait::async_trait;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcRequest};
use reqwest::Client as HttpClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::warn;

use super::{send_message, Error, PendingRequests, Transport, TransportHandle, TransportMessage};

/// HTTP transport for MCP that implements the POST-based portion of the Streamable HTTP spec
pub struct HttpTransport {
    endpoint_url: String,
    http_client: HttpClient,
    session_id: Arc<RwLock<Option<String>>>,
    custom_headers: HashMap<String, String>,
}

impl HttpTransport {
    pub fn new<S: Into<String>>(endpoint_url: S) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            http_client: HttpClient::new(),
            session_id: Arc::new(RwLock::new(None)),
            custom_headers: HashMap::new(),
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.custom_headers = headers;
        self
    }
}

#[derive(Clone)]
pub struct HttpTransportHandle {
    sender: mpsc::Sender<TransportMessage>,
    session_id: Arc<RwLock<Option<String>>>,
}

impl HttpTransportHandle {
    /// Get the current session ID if one exists
    pub async fn session_id(&self) -> Option<String> {
        self.session_id.read().await.clone()
    }
}

#[async_trait]
impl TransportHandle for HttpTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error> {
        send_message(&self.sender, message).await
    }
}

pub struct HttpActor {
    receiver: mpsc::Receiver<TransportMessage>,
    pending_requests: Arc<PendingRequests>,
    endpoint_url: String,
    http_client: HttpClient,
    session_id: Arc<RwLock<Option<String>>>,
    custom_headers: HashMap<String, String>,
}

impl HttpActor {
    pub async fn run(mut self) {
        while let Some(transport_msg) = self.receiver.recv().await {
            let result = self.handle_message(transport_msg).await;
            if let Err(e) = result {
                tracing::error!("Error handling message: {:?}", e);
            }
        }

        // Clean up pending requests when actor stops
        self.pending_requests.clear().await;
    }

    async fn handle_message(&self, mut transport_msg: TransportMessage) -> Result<(), Error> {
        // Serialize the message
        let message_str =
            serde_json::to_string(&transport_msg.message).map_err(Error::Serialization)?;

        // Store pending request if needed
        if let Some(response_tx) = transport_msg.response_tx.take() {
            if let JsonRpcMessage::Request(JsonRpcRequest { id: Some(id), .. }) =
                &transport_msg.message
            {
                self.pending_requests
                    .insert(id.to_string(), response_tx)
                    .await;
            }
        }

        // Build request
        let mut request = self
            .http_client
            .post(&self.endpoint_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(message_str);

        // Add custom headers
        for (key, value) in &self.custom_headers {
            request = request.header(key, value);
        }

        // Add session ID if present
        if let Some(session_id) = &*self.session_id.read().await {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Send request
        let response = request.send().await.map_err(|e| Error::HttpError {
            status: 0,
            message: e.to_string(),
        })?;

        // Handle response
        self.handle_response(response).await
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<(), Error> {
        let status = response.status();

        // Check for session ID in response headers
        if let Some(session_id) = response.headers().get("Mcp-Session-Id") {
            if let Ok(session_str) = session_id.to_str() {
                *self.session_id.write().await = Some(session_str.to_string());
            }
        }

        if !status.is_success() {
            return Err(Error::HttpError {
                status: status.as_u16(),
                message: status.to_string(),
            });
        }

        // Handle 202 Accepted (no body)
        if status.as_u16() == 202 {
            return Ok(());
        }

        // Check content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("application/json") {
            // Handle single JSON response
            let body = response.text().await.map_err(|e| Error::HttpError {
                status: 0,
                message: e.to_string(),
            })?;

            if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&body) {
                self.process_json_rpc_message(message).await;
            }
        } else if content_type.contains("text/event-stream") {
            // For now, we'll just log that SSE is not fully supported
            warn!("SSE response received, but full SSE support not implemented in this basic HTTP transport");
            // Future enhancement: parse SSE stream and handle messages
        }

        Ok(())
    }

    async fn process_json_rpc_message(&self, message: JsonRpcMessage) {
        match &message {
            JsonRpcMessage::Response(response) => {
                if let Some(id) = &response.id {
                    self.pending_requests
                        .respond(&id.to_string(), Ok(message))
                        .await;
                }
            }
            JsonRpcMessage::Error(error) => {
                if let Some(id) = &error.id {
                    self.pending_requests
                        .respond(&id.to_string(), Ok(message))
                        .await;
                }
            }
            _ => {
                // Handle other message types if needed
                tracing::debug!("Received non-response message: {:?}", message);
            }
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    type Handle = HttpTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        let (tx, rx) = mpsc::channel(32);

        let actor = HttpActor {
            receiver: rx,
            pending_requests: Arc::new(PendingRequests::new()),
            endpoint_url: self.endpoint_url.clone(),
            http_client: self.http_client.clone(),
            session_id: Arc::clone(&self.session_id),
            custom_headers: self.custom_headers.clone(),
        };

        tokio::spawn(actor.run());

        Ok(HttpTransportHandle {
            sender: tx,
            session_id: Arc::clone(&self.session_id),
        })
    }

    async fn close(&self) -> Result<(), Error> {
        // If we have a session ID, send DELETE request to terminate session
        if let Some(session_id) = &*self.session_id.read().await {
            let _ = self
                .http_client
                .delete(&self.endpoint_url)
                .header("Mcp-Session-Id", session_id)
                .send()
                .await;
        }
        Ok(())
    }
}
