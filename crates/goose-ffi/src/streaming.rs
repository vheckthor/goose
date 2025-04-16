use std::ffi::{c_char, CStr, CString};
use std::ptr;

use futures::StreamExt;
use goose::agents::Agent;
use goose::message::Message;
use mcp_core::{Content, ToolResult, Role};
use tracing::{debug, error, info, warn};

use crate::{get_runtime, string_to_c_char, AgentPtr, AsyncResult, MessageFFI};

/// Opaque pointer to StreamState
#[repr(C)]
pub struct StreamStatePtr(*mut StreamState);

/// Stream state for managing ongoing conversation
pub struct StreamState {
    agent: *mut Agent,
    stream: Option<futures::stream::BoxStream<'static, Result<Message, goose::providers::errors::ProviderError>>>,
}

/// Create a new stream state for an agent
///
/// This function creates a new stream state for an agent, which can be used
/// to manage an ongoing conversation with streaming responses.
///
/// Note: This function only creates the stream state container but does not
/// initialize an active stream. You must call goose_stream_send_message
/// before calling goose_stream_next to create an active stream.
///
/// # Parameters
///
/// - agent_ptr: Agent pointer
///
/// # Returns
///
/// A new stream state pointer, or NULL on error.
///
/// # Safety
///
/// The agent_ptr must be a valid pointer returned by goose_agent_new.
/// The resulting stream state must be freed with goose_stream_free when no longer needed.
#[no_mangle]
pub extern "C" fn goose_stream_new(
    agent_ptr: AgentPtr,
) -> StreamStatePtr {
    debug!("goose_stream_new called with agent_ptr={:?}", agent_ptr.0);
    
    if agent_ptr.0.is_null() {
        error!("goose_stream_new: agent_ptr is null");
        return StreamStatePtr(ptr::null_mut());
    }

    info!("Creating new stream state");
    let stream_state = Box::new(StreamState {
        agent: agent_ptr.0,
        stream: None,
    });
    
    let ptr = Box::into_raw(stream_state);
    debug!("Stream state created at {:?}", ptr);
    StreamStatePtr(ptr)
}

/// Free a stream state
///
/// This function frees the memory allocated for a stream state.
///
/// # Parameters
///
/// - stream_ptr: Stream state pointer
///
/// # Safety
///
/// The stream_ptr must be a valid pointer returned by goose_stream_new,
/// or have a null internal pointer. The stream_ptr must not be used after
/// calling this function.
#[no_mangle]
pub extern "C" fn goose_stream_free(stream_ptr: StreamStatePtr) {
    debug!("goose_stream_free called with stream_ptr={:?}", stream_ptr.0);
    
    if !stream_ptr.0.is_null() {
        info!("Freeing stream state at {:?}", stream_ptr.0);
        unsafe { 
            let _ = Box::from_raw(stream_ptr.0);
        }
        debug!("Stream state freed successfully");
    } else {
        warn!("Attempted to free null stream_ptr");
    }
}

/// Get the next message from the stream
///
/// This function gets the next message from the stream. If there are no more
/// messages, it returns NULL.
///
/// # Parameters
///
/// - stream_ptr: Stream state pointer
///
/// # Returns
///
/// A pointer to a MessageFFI struct, or NULL if there are no more messages, no active stream, or an error occurred.
/// The message must be freed with goose_free_message when no longer needed.
///
/// # Safety
///
/// The stream_ptr must be a valid pointer returned by goose_stream_new.
#[no_mangle]
pub extern "C" fn goose_stream_next(stream_ptr: StreamStatePtr) -> *mut MessageFFI {
    debug!("goose_stream_next called with stream_ptr={:?}", stream_ptr.0);
    
    if stream_ptr.0.is_null() {
        error!("goose_stream_next: stream_ptr is null");
        return ptr::null_mut();
    }

    let stream_state = unsafe { &mut *stream_ptr.0 };
    
    // If there's no active stream, return NULL
    // This can happen if goose_stream_send_message hasn't been called yet
    if stream_state.stream.is_none() {
        warn!("goose_stream_next: No active stream available");
        return ptr::null_mut();
    }
    
    let mut stream = stream_state.stream.take().unwrap();
    
    // Get the next message from the stream
    debug!("Getting next message from stream");
    let message_result = get_runtime().block_on(async {
        stream.next().await
    });
    
    // Put the stream back
    stream_state.stream = Some(stream);
    
    match message_result {
        Some(Ok(message)) => {
            debug!("Received message with role: {:?}", message.role);
            
            // Convert the message to JSON to preserve all information
            let json_message = match serde_json::to_string(&message) {
                Ok(json) => {
                    debug!("Serialized message to JSON (length: {})", json.len());
                    json
                },
                Err(e) => {
                    error!("Failed to serialize message to JSON: {}", e);
                    return ptr::null_mut();
                },
            };
            
            let role = match message.role {
                Role::User => 0,
                Role::Assistant => 1,
            };
            
            let message_ffi = Box::new(MessageFFI {
                role,
                content: string_to_c_char(&json_message),
            });
            
            let ptr = Box::into_raw(message_ffi);
            debug!("Created MessageFFI at {:?}", ptr);
            ptr
        },
        Some(Err(e)) => {
            error!("Error getting next message from stream: {}", e);
            ptr::null_mut()
        },
        None => {
            info!("No more messages in the stream");
            ptr::null_mut()
        },
    }
}

/// Submit a tool result to the stream
///
/// This function submits a tool result to the stream, which will be used by the agent
/// to continue the conversation.
///
/// # Parameters
///
/// - stream_ptr: Stream state pointer
/// - tool_id: Tool ID
/// - result_json: Tool result as JSON
///
/// # Returns
///
/// An AsyncResult struct with the result of the operation.
///
/// # Safety
///
/// The stream_ptr must be a valid pointer returned by goose_stream_new.
/// The tool_id and result_json must be valid C strings.
#[no_mangle]
pub extern "C" fn goose_stream_submit_tool_result(
    stream_ptr: StreamStatePtr,
    tool_id: *const c_char,
    result_json: *const c_char,
) -> *mut AsyncResult {
    debug!("goose_stream_submit_tool_result called with stream_ptr={:?}, tool_id={:?}, result_json={:?}", 
           stream_ptr.0, tool_id, result_json);
    
    if stream_ptr.0.is_null() || tool_id.is_null() || result_json.is_null() {
        error!("goose_stream_submit_tool_result: Invalid parameters (null pointers)");
        let result = Box::new(AsyncResult {
            succeeded: false,
            error_message: string_to_c_char("Invalid parameters"),
        });
        return Box::into_raw(result);
    }

    let stream_state = unsafe { &mut *stream_ptr.0 };
    let agent = unsafe { &mut *stream_state.agent };
    
    let tool_id = unsafe { CStr::from_ptr(tool_id).to_string_lossy().to_string() };
    let result_json = unsafe { CStr::from_ptr(result_json).to_string_lossy().to_string() };
    
    debug!("Tool ID: {}", tool_id);
    debug!("Result JSON: {}", result_json);
    
    // Parse the result JSON
    match serde_json::from_str::<serde_json::Value>(&result_json) {
        Ok(json_value) => {
            info!("Submitting tool result for tool ID: {}", tool_id);
            // Convert to Content
            let content = vec![Content::text(json_value.to_string())];
            let tool_result = ToolResult::Ok(content);
            
            // Use the agent's handle_tool_result method
            get_runtime().block_on(async {
                debug!("Calling agent.handle_tool_result");
                agent.handle_tool_result(tool_id, tool_result).await;
                debug!("agent.handle_tool_result completed");
            });
            
            let result = Box::new(AsyncResult {
                succeeded: true,
                error_message: ptr::null_mut(),
            });
            let ptr = Box::into_raw(result);
            debug!("Created AsyncResult at {:?}", ptr);
            ptr
        },
        Err(e) => {
            error!("Failed to parse result JSON: {}", e);
            let result = Box::new(AsyncResult {
                succeeded: false,
                error_message: string_to_c_char(&format!("Failed to parse result JSON: {}", e)),
            });
            Box::into_raw(result)
        },
    }
}

/// Free a message
///
/// This function frees the memory allocated for a message.
///
/// # Parameters
///
/// - message: Message pointer
///
/// # Safety
///
/// The message must be a valid pointer returned by goose_stream_next,
/// or NULL. The message must not be used after calling this function.
#[no_mangle]
pub extern "C" fn goose_free_message(message: *mut MessageFFI) {
    debug!("goose_free_message called with message={:?}", message);
    
    if !message.is_null() {
        let message = unsafe { &mut *message };
        if !message.content.is_null() {
            debug!("Freeing message content");
            unsafe {
                let _ = CString::from_raw(message.content as *mut c_char);
            }
        }
        debug!("Freeing message");
        unsafe {
            let _ = Box::from_raw(message);
        }
        debug!("Message freed successfully");
    } else {
        warn!("Attempted to free null message");
    }
}

/// Send a message to an ongoing stream
///
/// This function sends a message to an ongoing stream,
/// which will be used by the agent to continue the conversation.
/// If no stream exists yet, it will create a new one.
///
/// # Parameters
///
/// - stream_ptr: Stream state pointer
/// - message: Message to send
///
/// # Returns
///
/// An AsyncResult struct with the result of the operation.
///
/// # Safety
///
/// The stream_ptr must be a valid pointer returned by goose_stream_new.
/// The message must be a valid C string.
#[no_mangle]
pub extern "C" fn goose_stream_send_message(
    stream_ptr: StreamStatePtr,
    message: *const c_char,
) -> *mut AsyncResult {
    debug!("goose_stream_send_message called with stream_ptr={:?}, message={:?}", stream_ptr.0, message);
    
    if stream_ptr.0.is_null() || message.is_null() {
        error!("goose_stream_send_message: Invalid parameters (null pointers)");
        let result = Box::new(AsyncResult {
            succeeded: false,
            error_message: string_to_c_char("Invalid parameters"),
        });
        return Box::into_raw(result);
    }

    let stream_state = unsafe { &mut *stream_ptr.0 };
    let agent = unsafe { &mut *stream_state.agent };
    let message_text = unsafe { CStr::from_ptr(message).to_string_lossy().to_string() };
    
    debug!("Message text: {}", message_text);
    
    // Create a new user message
    let user_message = Message::user().with_text(&message_text);
    
    // Create a new stream with the message
    let messages = vec![user_message];
    
    info!("Sending message to agent");
    match get_runtime().block_on(agent.reply(&messages, None)) {
        Ok(stream) => {
            debug!("Successfully created reply stream");
            // Create a 'static stream by extending the lifetime
            // This is safe because we ensure the agent outlives the stream
            let stream: futures::stream::BoxStream<'static, Result<Message, goose::providers::errors::ProviderError>> = 
                unsafe { std::mem::transmute(stream) };
            
            // Replace the existing stream
            stream_state.stream = Some(stream);
            
            let result = Box::new(AsyncResult {
                succeeded: true,
                error_message: ptr::null_mut(),
            });
            let ptr = Box::into_raw(result);
            debug!("Created AsyncResult at {:?}", ptr);
            ptr
        },
        Err(e) => {
            error!("Failed to create reply stream: {}", e);
            let result = Box::new(AsyncResult {
                succeeded: false,
                error_message: string_to_c_char(&format!("Failed to create reply stream: {}", e)),
            });
            Box::into_raw(result)
        },
    }
}