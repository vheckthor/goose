// src/lib.rs

// 1) Proc-macro setup: no UDL, no build.rs
uniffi::setup_scaffolding!();

use thiserror::Error as ThisError;
use tokio::runtime::Builder;

use goose_llm::extractors::generate_tooltip as internal_generate_tooltip;
use goose_llm::message::Message as GooseMsg;
use goose_llm::providers::errors::ProviderError as InternalError;

// 2) Exported record type for FFI
#[derive(uniffi::Record)]
pub struct Message {
    pub role: String,
    pub text: String,
}

// 3) Your error enum, now deriving Debug + Display + UniFFI Error
#[derive(Debug, ThisError, uniffi::Error)]
pub enum ProviderError {
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Usage data error: {0}")]
    UsageError(String),

    #[error("Invalid response: {0}")]
    ResponseParseError(String),
}

// 4) Exported function for UniFFI
#[uniffi::export]
pub fn generate_tooltip(messages: Vec<Message>) -> Result<String, ProviderError> {
    // Map FFI → internal messages
    let internal: Vec<GooseMsg> = messages
        .into_iter()
        .map(|m| {
            let gm = if m.role.eq_ignore_ascii_case("assistant") {
                GooseMsg::assistant()
            } else {
                GooseMsg::user()
            };
            gm.with_text(&m.text)
        })
        .collect();

    // Run the async extractor on a minimal Tokio runtime
    let tooltip = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { internal_generate_tooltip(&internal).await });

    // Map your internal ProviderError → the UniFFI one
    tooltip.map_err(|e| match e {
        InternalError::Authentication(s) => ProviderError::Authentication(s),
        InternalError::ContextLengthExceeded(s) => ProviderError::ContextLengthExceeded(s),
        InternalError::RateLimitExceeded(s) => ProviderError::RateLimitExceeded(s),
        InternalError::ServerError(s) => ProviderError::ServerError(s),
        InternalError::RequestFailed(s) => ProviderError::RequestFailed(s),
        InternalError::ExecutionError(s) => ProviderError::ExecutionError(s),
        InternalError::UsageError(s) => ProviderError::UsageError(s),
        InternalError::ResponseParseError(s) => ProviderError::ResponseParseError(s),
    })
}
