use super::errors::ProviderError;
use super::oauth::{self, DEFAULT_REDIRECT_URL};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::formats::google::{create_request, get_usage, response_to_message};
use crate::providers::utils::{emit_debug_trace, unescape_json_values};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const GOOGLE_API_HOST: &str = "https://generativelanguage.googleapis.com";
pub const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_SCOPES: &[&str] = &["https://www.googleapis.com/auth/generative-language.retriever"];
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";
pub const GOOGLE_KNOWN_MODELS: &[&str] = &[
    "models/gemini-1.5-pro-latest",
    "models/gemini-1.5-pro",
    "models/gemini-1.5-flash-latest",
    "models/gemini-1.5-flash",
    "models/gemini-2.0-flash-exp",
    "models/gemini-2.0-flash-thinking-exp-01-21",
];

pub const GOOGLE_DOC_URL: &str = "https://ai.google/get-started/our-models/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoogleAuth {
    ApiKey(String),
    OAuth {
        client_id: String,
        client_secret: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl GoogleAuth {
    pub fn api_key(key: String) -> Self {
        Self::ApiKey(key)
    }

    pub fn oauth(client_id: String, client_secret: String) -> Self {
        Self::OAuth {
            client_id,
            client_secret,
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct GoogleProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    auth: GoogleAuth,
    model: ModelConfig,
}

impl Default for GoogleProvider {
    fn default() -> Self {
        let model = ModelConfig::new(GoogleProvider::metadata().default_model);
        GoogleProvider::from_env(model).expect("Failed to initialize Google provider")
    }
}

impl GoogleProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get("GOOGLE_HOST")
            .unwrap_or_else(|_| GOOGLE_API_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        // First try API key authentication
        if let Ok(api_key) = config.get_secret("GOOGLE_API_KEY") {
            return Ok(Self {
                client,
                host,
                auth: GoogleAuth::api_key(api_key),
                model,
            });
        }

        // Try OAuth authentication
        let client_id = config.get("GOOGLE_CLIENT_ID");
        let client_secret = config.get_secret("GOOGLE_CLIENT_SECRET");

        match (client_id, client_secret) {
            (Ok(id), Ok(secret)) => {
                let scopes = DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect();
                Ok(Self {
                    client,
                    host,
                    auth: GoogleAuth::OAuth {
                        client_id: id,
                        client_secret: secret,
                        redirect_url: DEFAULT_REDIRECT_URL.to_string(),
                        scopes,
                    },
                    model,
                })
            }
            _ => Err(anyhow::anyhow!(
                "Authentication configuration missing. Please set either GOOGLE_API_KEY or both GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET"
            )),
        }
    }

    async fn ensure_auth_header(&self) -> Result<String, ProviderError> {
        match &self.auth {
            GoogleAuth::ApiKey(key) => Ok(format!("Bearer {}", key)),
            GoogleAuth::OAuth {
                client_id,
                client_secret,
                scopes,
                ..  // Ignore redirect_url as we're using the default
            } => {
                let token = if client_secret.is_empty() {
                    // Use public client OAuth if no client secret
                    oauth::get_oauth_token_public_client_async(
                        GOOGLE_AUTH_ENDPOINT,
                        GOOGLE_TOKEN_ENDPOINT,
                        client_id,
                        scopes,
                    ).await
                } else {
                    // Use private client OAuth if client secret is present
                    oauth::get_oauth_token_with_endpoints_async(
                        GOOGLE_AUTH_ENDPOINT,
                        GOOGLE_TOKEN_ENDPOINT,
                        client_id,
                        client_secret,
                        scopes,
                    ).await
                };

                token
                    .map_err(|e| ProviderError::Authentication(format!("Failed to get OAuth token: {}", e)))
                    .map(|token| format!("Bearer {}", token))
            }
        }
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        let url = base_url
            .join(&format!(
                "v1beta/models/{}:generateContent",
                self.model.model_name,
            ))
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
            })?;

        let auth = self.ensure_auth_header().await?;

        // Add auth either as query param for API key or header for OAuth
        let mut request = self
            .client
            .post(url.to_string())
            .header("Content-Type", "application/json");

        match &self.auth {
            GoogleAuth::ApiKey(_) => {
                // Remove "Bearer " prefix for API key and pass as query param
                let api_key = auth.trim_start_matches("Bearer ").to_string();
                request = request.query(&[("key", api_key)]);
            }
            GoogleAuth::OAuth { .. } => {
                request = request.header("Authorization", auth);
            }
        }

        let response = request.json(&payload).send().await?;

        let status = response.status();
        let payload: Option<Value> = response.json().await.ok();

        match status {
            StatusCode::OK =>  payload.ok_or_else( || ProviderError::RequestFailed("Response body is not valid JSON".to_string()) ),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                    Status: {}. Response: {:?}", status, payload )))
            }
            StatusCode::BAD_REQUEST => {
                let mut error_msg = "Unknown error".to_string();
                if let Some(payload) = &payload {
                    if let Some(error) = payload.get("error") {
                        error_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error").to_string();
                        let error_status = error.get("status").and_then(|s| s.as_str()).unwrap_or("Unknown status");
                        if error_status == "INVALID_ARGUMENT" && error_msg.to_lowercase().contains("exceeds") {
                            return Err(ProviderError::ContextLengthExceeded(error_msg.to_string()));
                        }
                    }
                }
                tracing::debug!(
                    "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                );
                Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", status, error_msg)))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                Err(ProviderError::ServerError(format!("{:?}", payload)))
            }
            _ => {
                tracing::debug!(
                    "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                );
                Err(ProviderError::RequestFailed(format!("Request failed with status: {}", status)))
            }
        }
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "google",
            "Google Gemini",
            "Gemini models from Google AI",
            GOOGLE_DEFAULT_MODEL,
            GOOGLE_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            GOOGLE_DOC_URL,
            vec![
                ConfigKey::new("GOOGLE_API_KEY", false, true, None),
                ConfigKey::new("GOOGLE_HOST", false, false, Some(GOOGLE_API_HOST)),
                ConfigKey::new("GOOGLE_CLIENT_ID", false, false, None),
                ConfigKey::new("GOOGLE_CLIENT_SECRET", false, true, None),
                ConfigKey::new(
                    "GOOGLE_REDIRECT_URL",
                    false,
                    false,
                    Some(DEFAULT_REDIRECT_URL),
                ),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(&self.model, system, messages, tools)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(unescape_json_values(&response))?;
        let usage = get_usage(&response)?;
        let model = match response.get("modelVersion") {
            Some(model_version) => model_version.as_str().unwrap_or_default().to_string(),
            None => self.model.model_name.clone(),
        };
        emit_debug_trace(self, &payload, &response, &usage);
        let provider_usage = ProviderUsage::new(model, usage);
        Ok((message, provider_usage))
    }
}
