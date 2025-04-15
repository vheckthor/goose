use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::{Arc, Mutex};

use goose::agents::Agent;
use goose::message::{Message, ToolCall, ToolResponse};
use goose::model::{ModelConfig, ToolConfig};
use goose::providers::databricks::{DatabricksProvider, DATABRICKS_DEFAULT_MODEL};
use goose::tools::{Schema, ToolSchema};
use futures::StreamExt;
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use tokio::runtime::Runtime;

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

// Extension configuration will be implemented in a future commit

/// Message structure for agent interactions
///
/// - role: 0 = user, 1 = assistant, 2 = system
/// - content: Text content of the message
#[repr(C)]
pub struct MessageFFI {
    pub role: u32,
    pub content: *const c_char,
}

/// Tool parameter type enum
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ToolParamType {
    /// String parameter type
    String = 0,
    /// Number parameter type
    Number = 1,
    /// Boolean parameter type
    Boolean = 2,
    /// Array parameter type
    Array = 3,
    /// Object parameter type
    Object = 4,
}

/// Tool parameter requirement enum
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ToolParamRequirement {
    /// Parameter is required
    Required = 0,
    /// Parameter is optional
    Optional = 1,
}

/// Tool parameter definition used to define a tool schema
///
/// - name: Parameter name
/// - description: Parameter description
/// - param_type: Parameter type (0 = String, 1 = Number, 2 = Boolean, 3 = Array, 4 = Object)
/// - required: Whether the parameter is required (0 = Required, 1 = Optional)
#[repr(C)]
pub struct ToolParamDef {
    pub name: *const c_char,
    pub description: *const c_char,
    pub param_type: u32,
    pub required: u32,
}

/// Tool parameter value used when passing arguments to a tool callback
///
/// - name: Parameter name
/// - value: Parameter value as JSON string
#[repr(C)]
pub struct ToolParam {
    pub name: *const c_char,
    pub value: *const c_char,
}

/// Tool callback function type
///
/// Arguments:
/// - param_count: Number of parameters
/// - params: Array of parameters
/// - user_data: User-provided data pointer 
///
/// Returns: JSON string result (must be valid UTF-8), which must be freed by the caller
pub type ToolCallbackFn = extern "C" fn(
    param_count: usize,
    params: *const ToolParam,
    user_data: *mut c_void,
) -> *mut c_char;

/// Registered tool data
struct RegisteredTool {
    name: String,
    description: String,
    schema: ToolSchema,
    callback: ToolCallbackFn,
    user_data: *mut c_void,
}

// Global storage for registered tools
static TOOL_REGISTRY: OnceCell<Mutex<HashMap<String, RegisteredTool>>> = OnceCell::new();

// Get or initialize the global tool registry
fn get_tool_registry() -> &'static Mutex<HashMap<String, RegisteredTool>> {
    TOOL_REGISTRY.get_or_init(|| {
        Mutex::new(HashMap::new())
    })
}

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
pub extern "C" fn goose_agent_new(config: *const ProviderConfigFFI) -> AgentPtr {
    if config.is_null() {
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
    
    // Parse API key if provided, otherwise use environment variable
    let api_key = if config.api_key.is_null() {
        std::env::var("DATABRICKS_API_KEY").ok()
    } else {
        let key = unsafe { CStr::from_ptr(config.api_key).to_string_lossy().to_string() };
        std::env::set_var("DATABRICKS_API_KEY", &key);
        Some(key)
    };

    // Without an API key, we can't create a provider
    if api_key.is_none() {
        return AgentPtr(ptr::null_mut());
    }

    // Parse model name if provided or use default
    let model_name = if config.model_name.is_null() {
        DATABRICKS_DEFAULT_MODEL.to_string()
    } else {
        unsafe { CStr::from_ptr(config.model_name).to_string_lossy().to_string() }
    };

    // Parse host URL or use environment variable
    let host = if !config.host.is_null() {
        Some(unsafe { CStr::from_ptr(config.host).to_string_lossy().to_string() })
    } else {
        std::env::var("DATABRICKS_HOST").ok()
    };
    
    // Create model config with model name
    let model_config = ModelConfig::new(model_name);
    
    // Set host URL if provided
    if let Some(url) = host {
        // Add the host URL to the model configuration via environment variable
        // This ensures the provider will use it when created
        std::env::set_var("DATABRICKS_HOST", &url);
    }
    
    // Create Databricks provider
    match DatabricksProvider::from_env(model_config) {
        Ok(provider) => {
            let agent = Agent::new(Arc::new(provider));
            AgentPtr(Box::into_raw(Box::new(agent)))
        },
        Err(_) => AgentPtr(ptr::null_mut()),
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

/// Create a tool schema from parameter definitions
///
/// This function creates a JSON schema object for tool parameters.
///
/// # Parameters
///
/// - name: Tool name
/// - description: Tool description
/// - params: Array of parameter definitions
/// - param_count: Number of parameters
///
/// # Returns
///
/// A C string with the tool schema JSON, or NULL on error.
/// This string must be freed with goose_free_string when no longer needed.
///
/// # Safety
///
/// The name, description, and param pointers must be valid.
#[no_mangle]
pub extern "C" fn goose_create_tool_schema(
    name: *const c_char,
    description: *const c_char,
    params: *const ToolParamDef,
    param_count: usize,
) -> *mut c_char {
    if name.is_null() || description.is_null() || (params.is_null() && param_count > 0) {
        return ptr::null_mut();
    }

    let name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
    let description = unsafe { CStr::from_ptr(description).to_string_lossy().to_string() };
    
    // Create properties object for the schema
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    // Process each parameter definition
    for i in 0..param_count {
        let param = unsafe { &*params.add(i) };
        
        if param.name.is_null() || param.description.is_null() {
            continue; // Skip invalid parameters
        }
        
        let param_name = unsafe { CStr::from_ptr(param.name).to_string_lossy().to_string() };
        let param_description = unsafe { CStr::from_ptr(param.description).to_string_lossy().to_string() };
        
        // Determine parameter type
        let param_type = match param.param_type {
            0 => "string",
            1 => "number",
            2 => "boolean",
            3 => "array",
            4 => "object",
            _ => "string", // Default to string for unknown types
        };
        
        // Create property object
        let mut property = serde_json::Map::new();
        property.insert("type".to_string(), json!(param_type));
        property.insert("description".to_string(), json!(param_description));
        
        // Add to properties
        properties.insert(param_name.clone(), Value::Object(property));
        
        // Add to required list if necessary
        if param.required == 0 { // 0 = Required
            required.push(json!(param_name));
        }
    }
    
    // Create the schema object
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(properties));
    
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    
    // Create the tool schema object
    let tool_schema = json!({
        "name": name,
        "description": description,
        "parameters": schema,
    });
    
    // Convert to JSON string
    match serde_json::to_string(&tool_schema) {
        Ok(json_str) => string_to_c_char(&json_str),
        Err(_) => ptr::null_mut(),
    }
}

/// Register a tool callback for an agent
///
/// This function registers a tool callback for an agent. The callback will be called
/// when the agent invokes the tool.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
/// - name: Tool name
/// - description: Tool description
/// - schema_json: Tool schema JSON string (created with goose_create_tool_schema)
/// - callback: Tool callback function
/// - user_data: User data to pass to the callback
///
/// # Returns
///
/// true if successful, false otherwise
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The name, description, and schema_json must be valid UTF-8 C strings.
/// The callback must be a valid function pointer.
#[no_mangle]
pub extern "C" fn goose_agent_register_tool_callback(
    agent_ptr: AgentPtr,
    name: *const c_char,
    description: *const c_char,
    schema_json: *const c_char,
    callback: ToolCallbackFn,
    user_data: *mut c_void,
) -> bool {
    if agent_ptr.0.is_null() || name.is_null() || description.is_null() || schema_json.is_null() {
        return false;
    }

    let name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
    let description = unsafe { CStr::from_ptr(description).to_string_lossy().to_string() };
    let schema_str = unsafe { CStr::from_ptr(schema_json).to_string_lossy().to_string() };
    
    // Parse the schema
    let schema: Value = match serde_json::from_str(&schema_str) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    // Create tool schema
    let tool_schema = ToolSchema::new(name.clone(), schema);
    
    // Store the tool
    let registry = get_tool_registry();
    let mut registry = match registry.lock() {
        Ok(r) => r,
        Err(_) => return false,
    };
    
    registry.insert(name.clone(), RegisteredTool {
        name,
        description,
        schema: tool_schema,
        callback,
        user_data,
    });
    
    // Apply the tool to the agent
    let agent = unsafe { &mut *agent_ptr.0 };
    let tool_configs: Vec<ToolConfig> = registry.values()
        .map(|tool| ToolConfig {
            name: tool.name.clone(),
            description: tool.description.clone(),
            schema: tool.schema.clone(),
        })
        .collect();
    
    // Set tools for the agent
    if !tool_configs.is_empty() {
        // Block on the async call using our global runtime
        get_runtime().block_on(async {
            // This will overwrite any existing tools
            agent.set_tools(tool_configs).await
        });
    }
    
    true
}

/// Send a message to the agent and get the response
///
/// This function sends a message to the agent and returns the response.
/// If the agent invokes tools, the registered tool callbacks will be called.
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
        let mut tool_responses = Vec::new();
        
        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    // Check if there are tool calls in the message
                    if let Some(tool_calls) = message.tool_calls() {
                        for tool_call in tool_calls {
                            if let Some(tool_response) = handle_tool_call(tool_call) {
                                tool_responses.push(tool_response);
                            }
                        }
                        
                        // Send tool responses back to the agent if we have any
                        if !tool_responses.is_empty() {
                            match agent.respond_to_tools(&tool_responses).await {
                                Ok(_) => {},
                                Err(e) => full_response.push_str(&format!("\nError responding to tools: {}", e)),
                            }
                            
                            // Clear the tool responses after sending them
                            tool_responses.clear();
                        }
                    } else {
                        // Regular message, just append it to the response
                        if let Some(text) = message.text() {
                            full_response.push_str(text);
                        } else if let Ok(json) = serde_json::to_string(&message) {
                            full_response.push_str(&json);
                        }
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

// Handle a tool call by invoking the registered callback
fn handle_tool_call(tool_call: &ToolCall) -> Option<ToolResponse> {
    let registry = get_tool_registry();
    let registry = match registry.lock() {
        Ok(r) => r,
        Err(_) => return None,
    };
    
    // Get the tool name and arguments
    let tool_name = &tool_call.name;
    let tool_id = tool_call.id.clone();
    let arguments = tool_call.arguments.clone();
    
    // Find the registered tool
    let tool = match registry.get(tool_name) {
        Some(t) => t,
        None => return Some(ToolResponse::error(
            tool_id,
            tool_name.clone(),
            format!("Tool not found: {}", tool_name),
        )),
    };
    
    // Parse the arguments
    let args: Value = match serde_json::from_str(&arguments) {
        Ok(a) => a,
        Err(e) => return Some(ToolResponse::error(
            tool_id,
            tool_name.clone(),
            format!("Failed to parse arguments: {}", e),
        )),
    };
    
    // Extract the arguments as key-value pairs
    let args_obj = match args.as_object() {
        Some(o) => o,
        None => return Some(ToolResponse::error(
            tool_id,
            tool_name.clone(),
            "Arguments must be an object".to_string(),
        )),
    };
    
    // Convert arguments to ToolParam array
    let mut params = Vec::new();
    for (name, value) in args_obj {
        // Convert the value to a JSON string
        let value_str = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        // Create C strings
        let name_c = match CString::new(name.as_str()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        let value_c = match CString::new(value_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        params.push(ToolParam {
            name: name_c.into_raw(),
            value: value_c.into_raw(),
        });
    }
    
    // Call the callback function
    let param_count = params.len();
    let param_ptr = if param_count > 0 { params.as_ptr() } else { ptr::null() };
    
    let result_ptr = (tool.callback)(param_count, param_ptr, tool.user_data);
    
    // Free the parameters
    for param in params {
        unsafe {
            if !param.name.is_null() {
                let _ = CString::from_raw(param.name as *mut c_char);
            }
            if !param.value.is_null() {
                let _ = CString::from_raw(param.value as *mut c_char);
            }
        }
    }
    
    // Handle the result
    if result_ptr.is_null() {
        return Some(ToolResponse::error(
            tool_id,
            tool_name.clone(),
            "Tool callback returned NULL".to_string(),
        ));
    }
    
    // Convert the result to a string
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().to_string() };
    
    // Free the result
    unsafe {
        let _ = CString::from_raw(result_ptr);
    }
    
    // Create the tool response
    Some(ToolResponse::success(tool_id, tool_name.clone(), result_str))
}


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

// Helper functions for future extension support will be added in a later commit