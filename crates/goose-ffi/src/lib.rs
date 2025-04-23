use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::Arc;

use futures::StreamExt;
use goose::agents::Agent;
use goose::agents::extension::ExtensionConfig;
use goose::config::Config;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::databricks::DatabricksProvider;
use mcp_core::{Content, Tool, ToolResult};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use serde_json::{self, Value};

mod reply;
use reply::AgentReplyState;

// This class is in alpha and not yet ready for production use
// and the API is not yet stable. Use at your own risk.

// Thread-safe global runtime
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

// Get or initialize the global runtime
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        // Runtime with all features enabled
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Pointer type for the agent
pub type AgentPtr = *mut Agent;
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
/// - ephemeral: Whether to use ephemeral in-memory configuration (true) or persistent configuration (false)
#[repr(C)]
pub struct ProviderConfigFFI {
    pub provider_type: ProviderType,
    pub api_key: *const c_char,
    pub model_name: *const c_char,
    pub host: *const c_char,
    pub ephemeral: bool,
}

// Pointer type for agent reply state
pub type AgentReplyStatePtr = *mut AgentReplyState;

/// Role enum for message participants
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MessageRole {
    /// User message role
    User = 0,
    /// Assistant message role
    Assistant = 1,
    /// System message role
    System = 2,
}

/// Message structure for agent interactions
///
/// - role: Message role (User, Assistant, or System)
/// - content: Text content of the message
#[repr(C)]
pub struct MessageFFI {
    pub role: MessageRole,
    pub content: *const c_char,
}

/// Result status for reply step operations
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ReplyStatus {
    /// Reply is complete, no more steps needed
    Complete = 0,
    /// Tool call needed, waiting for tool result
    ToolCallNeeded = 1,
    /// Error occurred
    Error = 2,
}

/// Tool call information
#[repr(C)]
pub struct ToolCallFFI {
    pub id: *mut c_char,
    pub tool_name: *mut c_char,
    pub arguments_json: *mut c_char,
}

/// Reply step result
#[repr(C)]
pub struct ReplyStepResult {
    pub status: ReplyStatus,
    pub message: *mut c_char,
    pub tool_call: ToolCallFFI,
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
pub unsafe extern "C" fn goose_free_async_result(result: *mut AsyncResult) {
    if !result.is_null() {
        let result = &mut *result;
        if !result.error_message.is_null() {
            let _ = CString::from_raw(result.error_message);
        }
        let _ = Box::from_raw(result);
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
pub unsafe extern "C" fn goose_agent_new(config: *const ProviderConfigFFI) -> AgentPtr {
    // Check for null pointer
    if config.is_null() {
        eprintln!("Error: config pointer is null");
        return ptr::null_mut();
    }

    let config = &*config;

    // We currently only support Databricks provider
    // This match ensures future compiler errors if new provider types are added without handling
    match config.provider_type {
        ProviderType::Databricks => (), // Databricks provider is supported
    }

    // Build a per-agent Config: in-memory if requested, else default
    let cfg = if config.ephemeral {
        Config::new_in_memory()
    } else {
        Config::default()
    };

    // Get api_key from config or environment
    let api_key = if !config.api_key.is_null() {
        CStr::from_ptr(config.api_key).to_string_lossy().to_string()
    } else {
        match std::env::var("DATABRICKS_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Error: api_key not provided and DATABRICKS_API_KEY environment variable not set");
                return ptr::null_mut();
            }
        }
    };

    // Check and get required model_name (no env fallback for model)
    if config.model_name.is_null() {
        eprintln!("Error: model_name is required but was null");
        return ptr::null_mut();
    }
    let model_name = CStr::from_ptr(config.model_name)
        .to_string_lossy()
        .to_string();

    // Get host from config or environment
    let host = if !config.host.is_null() {
        CStr::from_ptr(config.host).to_string_lossy().to_string()
    } else {
        match std::env::var("DATABRICKS_HOST") {
            Ok(url) => url,
            Err(_) => {
                eprintln!(
                    "Error: host not provided and DATABRICKS_HOST environment variable not set"
                );
                return ptr::null_mut();
            }
        }
    };

    // Create model config with model name
    let model_config = ModelConfig::new(model_name);

    // First set GOOSE_MODE in the config
    if let Err(e) = cfg.set_param("GOOSE_MODE", Value::String("auto".to_string())) {
        eprintln!("Warning: Failed to set GOOSE_MODE: {:?}", e);
    }
    
    // Create Databricks provider with required parameters
    match DatabricksProvider::from_params(host, api_key, model_config) {
        Ok(provider) => {
            // Use per-agent Config rather than the global
            let agent = Agent::new_with_config(Arc::new(provider), cfg);
            Box::into_raw(Box::new(agent))
        }
        Err(e) => {
            eprintln!("Error creating Databricks provider: {:?}", e);
            ptr::null_mut()
        }
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
pub unsafe extern "C" fn goose_agent_free(agent_ptr: AgentPtr) {
    if !agent_ptr.is_null() {
        let _ = Box::from_raw(agent_ptr);
    }
}

/// Send a message to the agent and get the response
///
/// This function sends a message to the agent and returns the response.
/// Tool handling is not yet supported and will be implemented in a future commit
/// so this may change significantly
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
pub unsafe extern "C" fn goose_agent_send_message(
    agent_ptr: AgentPtr,
    message: *const c_char,
) -> *mut c_char {
    if agent_ptr.is_null() || message.is_null() {
        return ptr::null_mut();
    }

    let agent = &mut *agent_ptr;
    let message = CStr::from_ptr(message).to_string_lossy().to_string();

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
                }
                Err(e) => {
                    full_response.push_str(&format!("\nError in message stream: {}", e));
                }
            }
        }
        full_response
    });

    string_to_c_char(&response)
}

/// Begin a new non-streaming reply conversation with the agent
///
/// This function starts a new conversation and returns a state pointer that can be used
/// to continue the conversation step-by-step with goose_agent_reply_step
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
/// - message: Message to send
///
/// # Returns
///
/// A new agent reply state pointer, or NULL on error.
/// This pointer must be freed with goose_agent_reply_state_free when no longer needed.
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The message must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_reply_begin(
    agent_ptr: AgentPtr,
    message: *const c_char,
) -> AgentReplyStatePtr {
    if agent_ptr.is_null() {
        println!("ERROR: agent_ptr is null in goose_agent_reply_begin");
        return ptr::null_mut();
    }
    
    if message.is_null() {
        println!("ERROR: message is null in goose_agent_reply_begin");
        return ptr::null_mut();
    }

    let agent = &mut *agent_ptr;
    
    // Safely convert C string to Rust string
    let message_str = match CStr::from_ptr(message).to_str() {
        Ok(s) => {
            println!("DEBUG: Starting conversation with message: {}", s);
            s.to_string()
        },
        Err(e) => {
            println!("ERROR: Invalid UTF-8 in message: {}", e);
            return ptr::null_mut();
        }
    };
    
    let messages = vec![Message::user().with_text(&message_str)];

    println!("DEBUG: Creating new AgentReplyState");
    
    // Create initial state
    let state = match get_runtime().block_on(AgentReplyState::new(agent, messages)) {
        Ok(state) => {
            println!("DEBUG: AgentReplyState created successfully");
            state
        },
        Err(e) => {
            println!("ERROR: Failed to create AgentReplyState: {:?}", e);
            return ptr::null_mut();
        }
    };

    println!("DEBUG: Returning AgentReplyState pointer");
    Box::into_raw(Box::new(state))
}

/// Execute one step of the reply process
///
/// This function processes one step of the reply process. If the status is Complete,
/// the reply is done. If the status is ToolCallNeeded, the tool call information is
/// filled in and the caller should execute the tool and provide the result with
/// goose_agent_reply_tool_result.
///
/// # Parameters
///
/// - state_ptr: Agent reply state pointer
///
/// # Returns
///
/// A ReplyStepResult struct with the status, message, and tool call information.
/// The message and tool call fields must be freed with goose_free_string when
/// no longer needed.
///
/// # Safety
///
/// The state_ptr must be a valid pointer returned by goose_agent_reply_begin
/// or goose_agent_reply_tool_result.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_reply_step(state_ptr: AgentReplyStatePtr) -> ReplyStepResult {
    if state_ptr.is_null() {
        println!("ERROR: state_ptr is null in goose_agent_reply_step");
        return ReplyStepResult {
            status: ReplyStatus::Error,
            message: string_to_c_char("Error: state pointer is null"),
            tool_call: ToolCallFFI {
                id: ptr::null_mut(),
                tool_name: ptr::null_mut(),
                arguments_json: ptr::null_mut(),
            },
        };
    }

    println!("DEBUG: Processing reply step");
    let state = &mut *state_ptr;

    // Process one step
    let step_result = match get_runtime().block_on(state.step()) {
        Ok(result) => {
            println!("DEBUG: Step completed successfully");
            result
        },
        Err(e) => {
            println!("ERROR: Error in step execution: {}", e);
            return ReplyStepResult {
                status: ReplyStatus::Error,
                message: string_to_c_char(&format!("Error processing step: {}", e)),
                tool_call: ToolCallFFI {
                    id: ptr::null_mut(),
                    tool_name: ptr::null_mut(),
                    arguments_json: ptr::null_mut(),
                },
            };
        }
    };

    match step_result {
        reply::StepResult::Complete(msg) => {
            println!("DEBUG: Step returned Complete result");
            let json = match serde_json::to_string(&msg) {
                Ok(json) => {
                    println!("DEBUG: Message serialized successfully");
                    println!("DEBUG: Message JSON (first 100 chars): {}", json);
                    json
                },
                Err(e) => {
                    println!("ERROR: Failed to serialize message: {}", e);
                    format!("Error serializing message: {}", e)
                }
            };

            ReplyStepResult {
                status: ReplyStatus::Complete,
                message: string_to_c_char(&json),
                tool_call: ToolCallFFI {
                    id: ptr::null_mut(),
                    tool_name: ptr::null_mut(),
                    arguments_json: ptr::null_mut(),
                },
            }
        }
        reply::StepResult::ToolCallNeeded(request) => {
            println!("DEBUG: Step returned ToolCallNeeded, request ID: {}", request.id);
            let tool_call_result = &request.tool_call;

            match tool_call_result {
                Ok(tool_call) => {
                    println!("DEBUG: Tool call requested for: {}", tool_call.name);
                    
                    // Safely serialize arguments
                    let json = match serde_json::to_string(&tool_call.arguments) {
                        Ok(json) => {
                            println!("DEBUG: Arguments serialized successfully");
                            json
                        },
                        Err(e) => {
                            println!("ERROR: Failed to serialize arguments: {}", e);
                            "{}".to_string()
                        }
                    };

                    ReplyStepResult {
                        status: ReplyStatus::ToolCallNeeded,
                        message: ptr::null_mut(),
                        tool_call: ToolCallFFI {
                            id: string_to_c_char(&request.id),
                            tool_name: string_to_c_char(&tool_call.name),
                            arguments_json: string_to_c_char(&json),
                        },
                    }
                }
                Err(e) => {
                    let error_msg = format!("Tool call error: {}", e);
                    println!("ERROR: {}", error_msg);
                    
                    ReplyStepResult {
                        status: ReplyStatus::Error,
                        message: string_to_c_char(&error_msg),
                        tool_call: ToolCallFFI {
                            id: string_to_c_char(&request.id),
                            tool_name: ptr::null_mut(),
                            arguments_json: ptr::null_mut(),
                        },
                    }
                },
            }
        }
    }
}

/// Provide a tool result to continue the reply process
///
/// This function provides a tool result to the agent and continues the reply process.
/// It returns a new state pointer that can be used to continue the conversation.
///
/// # Parameters
///
/// - state_ptr: Agent reply state pointer
/// - tool_id: Tool ID from the previous step
/// - result: Tool result
///
/// # Returns
///
/// A new agent reply state pointer, or NULL on error.
/// This pointer must be freed with goose_agent_reply_state_free when no longer needed.
///
/// # Safety
///
/// The state_ptr must be a valid pointer returned by goose_agent_reply_begin
/// or goose_agent_reply_tool_result.
/// The tool_id and result must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_reply_tool_result(
    state_ptr: AgentReplyStatePtr,
    tool_id: *const c_char,
    result: *const c_char,
) -> AgentReplyStatePtr {
    if state_ptr.is_null() || tool_id.is_null() || result.is_null() {
        eprintln!("Error: Null pointer passed to goose_agent_reply_tool_result");
        return ptr::null_mut();
    }

    println!("DEBUG: Processing tool result");
    
    let state = &mut *state_ptr;
    
    // Get tool_id as string
    let tool_id_str = match CStr::from_ptr(tool_id).to_str() {
        Ok(s) => {
            println!("DEBUG: Tool ID: {}", s);
            s.to_string()
        },
        Err(e) => {
            eprintln!("Error: tool_id is not valid UTF-8: {}", e);
            return ptr::null_mut();
        }
    };
    
    // Get result as string
    let result_str = match CStr::from_ptr(result).to_str() {
        Ok(s) => {
            println!("DEBUG: Tool result: {}", s);
            s.to_string()
        },
        Err(e) => {
            eprintln!("Error: result is not valid UTF-8: {}", e);
            return ptr::null_mut();
        }
    };

    // Create tool result
    let tool_result: ToolResult<Vec<Content>> = Ok(vec![Content::text(result_str)]);

    // Apply tool result
    println!("DEBUG: Applying tool result to state");
    if let Err(e) = get_runtime().block_on(state.apply_tool_result(tool_id_str, tool_result)) {
        eprintln!("Error applying tool result: {:?}", e);
        return ptr::null_mut();
    }

    println!("DEBUG: Tool result applied successfully");
    
    // Return the same state pointer
    state_ptr
}

/// Free an agent reply state
///
/// This function frees the memory allocated for an agent reply state.
///
/// # Parameters
///
/// - state_ptr: Agent reply state pointer
///
/// # Safety
///
/// The state_ptr must be a valid pointer returned by goose_agent_reply_begin
/// or goose_agent_reply_tool_result.
/// The state_ptr must not be used after calling this function.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_reply_state_free(state_ptr: AgentReplyStatePtr) {
    if !state_ptr.is_null() {
        let _ = Box::from_raw(state_ptr);
    }
}

/// Free a tool call
///
/// This function frees the memory allocated for a tool call.
///
/// # Parameters
///
/// - tool_call: Tool call to free
///
/// # Safety
///
/// The tool_call must have been allocated by a goose FFI function.
/// The tool_call must not be used after calling this function.
#[no_mangle]
pub unsafe extern "C" fn goose_free_tool_call(mut tool_call: ToolCallFFI) {
    println!("DEBUG: Freeing tool call resources");
    
    if !tool_call.id.is_null() {
        println!("DEBUG: Freeing tool call ID");
        goose_free_string(tool_call.id);
        tool_call.id = ptr::null_mut();
    }
    
    if !tool_call.tool_name.is_null() {
        println!("DEBUG: Freeing tool name");
        goose_free_string(tool_call.tool_name);
        tool_call.tool_name = ptr::null_mut();
    }
    
    if !tool_call.arguments_json.is_null() {
        println!("DEBUG: Freeing arguments JSON");
        goose_free_string(tool_call.arguments_json);
        tool_call.arguments_json = ptr::null_mut();
    }
    
    println!("DEBUG: Tool call resources freed");
}

/// Register tools with the agent
///
/// This function registers tools with the agent for use with the non-streaming API.
/// The tools should be provided as a JSON array of Tool objects.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
/// - tools_json: JSON string containing an array of Tool objects
/// - extension_name: Optional name for the extension. If NULL, a default name will be used.
/// - instructions: Optional instructions for using the tools. If NULL, default instructions will be used.
///
/// # Returns
///
/// A boolean indicating success (true) or failure (false)
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The tools_json must be a valid JSON string in the expected format.
/// The extension_name and instructions must be valid UTF-8 strings or NULL.
#[no_mangle]
pub unsafe extern "C" fn goose_agent_register_tools(
    agent_ptr: AgentPtr,
    tools_json: *const c_char,
    extension_name: *const c_char,
    instructions: *const c_char,
) -> bool {
    if agent_ptr.is_null() || tools_json.is_null() {
        eprintln!("Error: agent_ptr or tools_json is null");
        return false;
    }

    let agent = &mut *agent_ptr;
    
    println!("DEBUG: Starting tool registration process");
    
    let tools_json_str = match CStr::from_ptr(tools_json).to_str() {
        Ok(s) => {
            println!("DEBUG: Successfully converted tools_json to UTF-8 string, length: {}", s.len());
            if s.len() > 100 {
                println!("DEBUG: tools_json (first 100 chars): {}", &s[..100]);
            } else {
                println!("DEBUG: tools_json: {}", s);
            }
            s
        },
        Err(e) => {
            eprintln!("Error: tools_json is not valid UTF-8: {}", e);
            return false;
        }
    };
    
    // Parse tools from JSON
    let tools: Vec<Tool> = match serde_json::from_str::<Vec<Tool>>(tools_json_str) {
        Ok(tools) => {
            println!("DEBUG: Successfully parsed {} tools from JSON", tools.len());
            tools
        },
        Err(e) => {
            eprintln!("Error parsing tools JSON: {}", e);
            return false;
        }
    };
    
    // Get extension name and instructions or use defaults
    let ext_name = if extension_name.is_null() {
        println!("DEBUG: Using default extension name");
        "kotlin_ffi_tools".to_string()
    } else {
        match CStr::from_ptr(extension_name).to_str() {
            Ok(s) => {
                println!("DEBUG: Using provided extension name: {}", s);
                s.to_string()
            },
            Err(e) => {
                eprintln!("Error: extension_name is not valid UTF-8: {}", e);
                return false;
            }
        }
    };
    
    let ext_instructions = if instructions.is_null() {
        println!("DEBUG: Using default instructions");
        Some("These tools are provided by an external service through FFI".to_string())
    } else {
        match CStr::from_ptr(instructions).to_str() {
            Ok(s) => {
                println!("DEBUG: Using provided instructions");
                Some(s.to_string())
            },
            Err(e) => {
                eprintln!("Error: instructions is not valid UTF-8: {}", e);
                return false;
            }
        }
    };
    
    // Create frontend extension
    let frontend_extension = ExtensionConfig::Frontend {
        name: ext_name,
        tools,
        instructions: ext_instructions,
        bundled: Some(false),
    };
    
    println!("DEBUG: Created extension configuration, attempting to add to agent");
    
    // Add extension to agent
    match get_runtime().block_on(agent.add_extension(frontend_extension)) {
        Ok(_) => {
            println!("DEBUG: Successfully registered tools with agent");
            true
        },
        Err(e) => {
            eprintln!("Error registering tools: {}", e);
            false
        }
    }
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
pub unsafe extern "C" fn goose_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

// Helper function to convert a Rust string to a C char pointer
fn string_to_c_char(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
