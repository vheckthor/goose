use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::Arc;

use futures::StreamExt;
use goose::agents::Agent;
use goose::agents::extension::ExtensionConfig;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::databricks::DatabricksProvider;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use serde_json;

mod streaming;

// Re-export streaming functions
pub use streaming::{
    StreamStatePtr,
    goose_stream_new,
    goose_stream_free,
    goose_stream_next,
    goose_stream_submit_tool_result,
    goose_free_message,
    goose_stream_send_message,
};

// Thread-safe global runtime
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

// Get or initialize the global runtime
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        // Runtime with all features enabled
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Opaque pointer to Agent
#[repr(C)]
pub struct AgentPtr(*mut Agent);

/// Provider Type enumeration
/// Currently only Databricks is supported
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ProviderType {
    /// Databricks AI provider
    Databricks = 0,
}

/// Provider configuration used to initialize an AI provider
///
/// - provider_type: Provider type (0 = Databricks, other values will produce an error)
/// - api_key: Provider API key (null for default from environment variables)
/// - model_name: Model name to use (null for provider default)
/// - host: Provider host URL (null for default from environment variables)
#[repr(C)]
pub struct ProviderConfigFFI {
    pub provider_type: u32,
    pub api_key: *const c_char,
    pub model_name: *const c_char,
    pub host: *const c_char,
}

/// Extension configuration used to initialize an extension for an agent
///
/// - name: Extension name
/// - config_json: JSON configuration for the extension (null for default)
#[repr(C)]
pub struct ExtensionConfigFFI {
    pub name: *const c_char,
    pub config_json: *const c_char,
}

/// Message structure for agent interactions
///
/// - role: 0 = user, 1 = assistant, 2 = system
/// - content: Text content of the message
#[repr(C)]
pub struct MessageFFI {
    pub role: u32,
    pub content: *const c_char,
}

// Tool callbacks will be implemented in a future commit

/// Result type for async operations
///
/// - succeeded: true if the operation succeeded, false otherwise
/// - error_message: Error message if succeeded is false, NULL otherwise
#[repr(C)]
pub struct AsyncResult {
    pub succeeded: bool,
    pub error_message: *mut c_char,
}

/// Free an async result structure
///
/// This function frees the memory allocated for an AsyncResult structure,
/// including any error message it contains.
///
/// # Safety
///
/// The result pointer must be a valid pointer returned by a goose FFI function,
/// or NULL.
#[no_mangle]
pub extern "C" fn goose_free_async_result(result: *mut AsyncResult) {
    if !result.is_null() {
        let result = unsafe { &mut *result };
        if !result.error_message.is_null() {
            unsafe {
                let _ = CString::from_raw(result.error_message);
            }
        }
        unsafe {
            let _ = Box::from_raw(result);
        }
    }
}

/// Create a new agent with the given provider configuration
///
/// # Parameters
///
/// - config: Provider configuration
/// - extension_config: Extension configuration (can be NULL if no extension is needed)
///
/// # Returns
///
/// A new agent pointer, or a null pointer if creation failed
///
/// # Safety
///
/// The config pointer must be valid or NULL. The resulting agent must be freed
/// with goose_agent_free when no longer needed.
#[no_mangle]
pub extern "C" fn goose_agent_new(
    config: *const ProviderConfigFFI,
    extension_config: *const ExtensionConfigFFI
) -> AgentPtr {
    // Check for null pointer
    println!("DEBUG: goose_agent_new called with config={:?}, extension_config={:?}", 
             config, extension_config);
    if config.is_null() {
        eprintln!("Error: config pointer is null");
        return AgentPtr(ptr::null_mut());
    }

    let config = unsafe { &*config };

    // Check if the provider type is supported
    match config.provider_type {
        0 => { /* Databricks provider is supported */ },
        unsupported => {
            eprintln!("Unsupported provider type: {}. Currently only Databricks (0) is supported.", unsupported);
            return AgentPtr(ptr::null_mut());
        }
    }

    // Get api_key from config or environment
    let api_key = if !config.api_key.is_null() {
        unsafe {
            CStr::from_ptr(config.api_key)
                .to_string_lossy()
                .to_string()
        }
    } else {
        match std::env::var("DATABRICKS_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Error: api_key not provided and DATABRICKS_API_KEY environment variable not set");
                return AgentPtr(ptr::null_mut());
            }
        }
    };

    // Check and get required model_name (no env fallback for model)
    if config.model_name.is_null() {
        eprintln!("Error: model_name is required but was null");
        return AgentPtr(ptr::null_mut());
    }
    let model_name = unsafe {
        CStr::from_ptr(config.model_name)
            .to_string_lossy()
            .to_string()
    };

    // Get host from config or environment
    let host = if !config.host.is_null() {
        unsafe {
            CStr::from_ptr(config.host)
                .to_string_lossy()
                .to_string()
        }
    } else {
        match std::env::var("DATABRICKS_HOST") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("Error: host not provided and DATABRICKS_HOST environment variable not set");
                return AgentPtr(ptr::null_mut());
            }
        }
    };

    // Create model config with model name
    let model_config = ModelConfig::new(model_name);

    // Create Databricks provider with required parameters
    match DatabricksProvider::from_params(host, api_key, model_config) {
        Ok(provider) => {
            let mut agent = Agent::new(Arc::new(provider));
            
    // Process extension configuration if provided
            if !extension_config.is_null() {
                println!("DEBUG: Extension config pointer is not null");
                let ext_config = unsafe { &*extension_config };
                
                // Debug the extension config
                if ext_config.name.is_null() {
                    println!("DEBUG: Extension name is null");
                } else {
                    let name = unsafe {
                        CStr::from_ptr(ext_config.name)
                            .to_string_lossy()
                            .to_string()
                    };
                    println!("DEBUG: Extension name: '{}'", name);
                    
                    let config_json = if !ext_config.config_json.is_null() {
                        let config_str = unsafe {
                            CStr::from_ptr(ext_config.config_json)
                                .to_string_lossy()
                                .to_string()
                        };
                        println!("DEBUG: Extension config JSON: '{}'", config_str);
                        Some(config_str)
                    } else {
                        println!("DEBUG: Extension config JSON is null");
                        None
                    };

                    // Try to parse the config JSON and create a frontend extension config
                    if let Some(config_str) = &config_json {
                        println!("DEBUG: Attempting to create frontend extension for '{}'", name);
                        
                        // Try to parse the config JSON
                        match serde_json::from_str::<serde_json::Value>(config_str) {
                            Ok(json_value) => {
                                println!("DEBUG: Successfully parsed JSON for extension '{}'", name);
                                println!("DEBUG: JSON content: {}", json_value);
                                
                                // Check if we have a "type" field to determine the extension type
                                let ext_type = json_value.get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("frontend"); // Default to frontend type
                                
                                println!("DEBUG: Extension type: {}", ext_type);
                                
                                if ext_type == "frontend" {
                                    // For frontend extensions, we need to extract tools and instructions
                                    let tools_value = json_value.get("tools");
                                    let instructions = json_value.get("instructions")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("A frontend extension");
                                    
                                    if tools_value.is_some() {
                                        // Create a new JSON object with the required structure
                                        // The ExtensionConfig is likely using a tag-based enum serialization
                                        // where the variant name is used as a field name
                                        let ext_json = serde_json::json!({
                                            "name": name,
                                            "tools": tools_value,
                                            "instructions": instructions,
                                            "bundled": false,
                                            "type": "frontend"
                                        });
                                        
                                        println!("DEBUG: Created extension JSON: {}", ext_json);
                                        
                                        // Try to deserialize to ExtensionConfig
                                        match serde_json::from_value::<ExtensionConfig>(ext_json.clone()) {
                                            Ok(extension_config) => {
                                                println!("DEBUG: Successfully created extension config for '{}'", name);
                                                
                                                // Use the agent's add_extension method
                                                if let Err(e) = get_runtime().block_on(agent.add_extension(extension_config)) {
                                                    eprintln!("Error adding extension {}: {:?}", name, e);
                                                } else {
                                                    println!("DEBUG: Successfully added extension '{}'", name);
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("Error creating extension config for {}: {}", name, e);
                                                eprintln!("JSON was: {}", ext_json);
                                            }
                                        }
                                    } else {
                                        eprintln!("Error: 'tools' field is required for frontend extension {}", name);
                                    }
                                } else {
                                    eprintln!("Error: Only 'frontend' extension type is supported, got '{}'", ext_type);
                                }
                            },
                            Err(e) => {
                                eprintln!("Error parsing extension config JSON for {}: {}", name, e);
                                eprintln!("JSON was: {}", config_str);
                            }
                        }
                    } else {
                        eprintln!("Error: Config JSON is required for frontend extensions");
                    }
                }
            } else {
                println!("DEBUG: Extension config pointer is null");
            }
            
            AgentPtr(Box::into_raw(Box::new(agent)))
        },
        Err(e) => {
            eprintln!("Error creating Databricks provider: {:?}", e);
            AgentPtr(ptr::null_mut())
        },
    }
}

/// Free an agent
///
/// This function frees the memory allocated for an agent.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer returned by goose_agent_new
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new,
/// or have a null internal pointer. The agent_ptr must not be used after
/// calling this function.
#[no_mangle]
pub extern "C" fn goose_agent_free(agent_ptr: AgentPtr) {
    if !agent_ptr.0.is_null() {
        unsafe { 
            let _ = Box::from_raw(agent_ptr.0);
        }
    }
}

// Extension functionality is handled during agent creation

// Tool callback registration will be implemented in a future commit

/// Send a message to the agent and get the response
///
/// This function sends a message to the agent and returns the response.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
/// - message: Message to send
///
/// # Returns
///
/// A C string with the agent's response, or NULL on error.
/// This string must be freed with goose_free_string when no longer needed.
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The message must be a valid C string.
#[no_mangle]
pub extern "C" fn goose_agent_send_message(
    agent_ptr: AgentPtr,
    message: *const c_char,
) -> *mut c_char {
    if agent_ptr.0.is_null() || message.is_null() {
        return ptr::null_mut();
    }

    let agent = unsafe { &mut *agent_ptr.0 };
    let message = unsafe { CStr::from_ptr(message).to_string_lossy().to_string() };
    
    let messages = vec![Message::user().with_text(&message)];

    // Block on the async call using our global runtime
    let response = get_runtime().block_on(async {
        let mut stream = match agent.reply(&messages, None).await {
            Ok(stream) => stream,
            Err(e) => return format!("Error getting reply from agent: {}", e),
        };

        let mut full_response = String::new();
        
        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    // Get text or serialize to JSON
                    // Note: Message doesn't have as_text method, we'll serialize to JSON
                    if let Ok(json) = serde_json::to_string(&message) {
                        full_response.push_str(&json);
                    }
                },
                Err(e) => {
                    full_response.push_str(&format!("\nError in message stream: {}", e));
                }
            }
        }
        full_response
    });

    string_to_c_char(&response)
}

// Tool schema creation will be implemented in a future commit

/// Free a string allocated by goose FFI functions
///
/// This function frees memory allocated for strings returned by goose FFI functions.
///
/// # Parameters
///
/// - s: String to free
///
/// # Safety
///
/// The string must have been allocated by a goose FFI function, or be NULL.
/// The string must not be used after calling this function.
#[no_mangle]
pub extern "C" fn goose_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// Helper function to convert a Rust string to a C char pointer
fn string_to_c_char(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}