#!/usr/bin/env python3
"""
Python example for using the Goose FFI interface.

This example demonstrates how to:
1. Load the Goose FFI library
2. Create an agent with a provider
3. Use the streaming API to send messages and get responses
4. Use the non-streaming API to send messages, handle tool calls, and get responses
"""

import ctypes
import json
import os
import platform
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER
from enum import IntEnum

# Platform-specific dynamic lib name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
elif platform.system() == "Windows":
    LIB_NAME = "goose_ffi.dll"
else:
    raise RuntimeError("Unsupported platform")

# Adjust to your actual build output directory
LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "debug", LIB_NAME)

# Load library
goose = ctypes.CDLL(LIB_PATH)

# Provider type enumeration
class ProviderType(IntEnum):
    DATABRICKS = 0

# Reply status enumeration
class ReplyStatus(IntEnum):
    COMPLETE = 0
    TOOL_CALL_NEEDED = 1
    ERROR = 2

# C struct mappings
class ProviderConfig(Structure):
    _fields_ = [
        ("provider_type", c_uint32),
        ("api_key", c_char_p),
        ("model_name", c_char_p),
        ("host", c_char_p),
        ("ephemeral", c_bool),
    ]

class AsyncResult(Structure):
    _fields_ = [
        ("succeeded", c_bool),
        ("error_message", c_char_p),
    ]

class ToolCallFFI(Structure):
    _fields_ = [
        ("id", c_char_p),
        ("tool_name", c_char_p),
        ("arguments_json", c_char_p),
    ]
    
class ReplyStepResult(Structure):
    _fields_ = [
        ("status", c_uint32),
        ("message", c_char_p),
        ("tool_call", ToolCallFFI),
    ]

# Forward declaration for goose_Agent and AgentReplyState
class goose_Agent(Structure):
    pass

class AgentReplyState(Structure):
    pass

# Pointer types
goose_AgentPtr = POINTER(goose_Agent)
AgentReplyStatePtr = POINTER(AgentReplyState)

# Function signatures - Streaming API
goose.goose_agent_new.argtypes = [POINTER(ProviderConfig)]
goose.goose_agent_new.restype = goose_AgentPtr

goose.goose_agent_free.argtypes = [goose_AgentPtr]
goose.goose_agent_free.restype = None

goose.goose_agent_send_message.argtypes = [goose_AgentPtr, c_char_p]
goose.goose_agent_send_message.restype = c_void_p

# Function signatures - Non-streaming API
goose.goose_agent_reply_begin.argtypes = [goose_AgentPtr, c_char_p]
goose.goose_agent_reply_begin.restype = c_void_p

goose.goose_agent_reply_step.argtypes = [c_void_p]
goose.goose_agent_reply_step.restype = ReplyStepResult

goose.goose_agent_reply_tool_result.argtypes = [c_void_p, c_char_p, c_char_p]
goose.goose_agent_reply_tool_result.restype = c_void_p

goose.goose_agent_reply_state_free.argtypes = [c_void_p]
goose.goose_agent_reply_state_free.restype = None

goose.goose_free_tool_call.argtypes = [ToolCallFFI]
goose.goose_free_tool_call.restype = None

goose.goose_free_string.argtypes = [c_void_p]
goose.goose_free_string.restype = None

goose.goose_free_async_result.argtypes = [POINTER(AsyncResult)]
goose.goose_free_async_result.restype = None

def execute_tool(tool_name, args):
    """Execute a tool based on its name and arguments."""
    print(f"Executing tool: {tool_name}")
    print(f"Arguments: {args}")
    
    if tool_name == "calculator":
        try:
            result = eval(str(args.get("expression", "0")))
            return f"The result is: {result}"
        except Exception as e:
            return f"Error calculating: {e}"
    elif tool_name == "weather":
        location = args.get("location", "unknown")
        return f"Weather in {location} is currently sunny, 72°F."
    else:
        return f"Unknown tool: {tool_name}"

class GooseAgent:
    def __init__(self, provider_type=ProviderType.DATABRICKS, api_key=None, model_name=None, host=None, ephemeral=False):
        self.config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key.encode("utf-8") if api_key else None,
            model_name=model_name.encode("utf-8") if model_name else None,
            host=host.encode("utf-8") if host else None,
            ephemeral=ephemeral,
        )
        self.agent = goose.goose_agent_new(ctypes.byref(self.config))
        if not self.agent:
            raise RuntimeError("Failed to create Goose agent")

    def __del__(self):
        if getattr(self, "agent", None):
            goose.goose_agent_free(self.agent)

    def send_message(self, message: str) -> str:
        """Send a message using the streaming API."""
        msg = message.encode("utf-8")
        response_ptr = goose.goose_agent_send_message(self.agent, msg)
        if not response_ptr:
            return "Error or NULL response from agent"
        response = ctypes.string_at(response_ptr).decode("utf-8")
        # Free the string using the proper C function provided by the library
        goose.goose_free_string(response_ptr)
        return response
    
    def send_message_non_streaming(self, message: str) -> str:
        """Send a message using the non-streaming API with tool handling."""
        msg = message.encode("utf-8")
        
        # Begin reply
        reply_state = goose.goose_agent_reply_begin(self.agent, msg)
        if not reply_state:
            return "Error starting reply"
            
        try:
            response_text = ""
            
            # Process steps until complete
            while True:
                result = goose.goose_agent_reply_step(reply_state)
                
                if result.status == ReplyStatus.ERROR:
                    error_message = ctypes.string_at(result.message).decode('utf-8')
                    goose.goose_free_string(result.message)
                    return f"Error: {error_message}"
                    
                elif result.status == ReplyStatus.COMPLETE:
                    # Get the final message
                    message_json = ctypes.string_at(result.message).decode('utf-8')
                    try:
                        message = json.loads(message_json)
                        
                        # Extract text content
                        text_parts = []
                        for content in message.get("content", []):
                            if content.get("type") == "text":
                                text_parts.append(content.get("text", ""))
                                
                        response_text = "\n".join(text_parts)
                    except json.JSONDecodeError:
                        response_text = message_json
                        
                    goose.goose_free_string(result.message)
                    break
                    
                elif result.status == ReplyStatus.TOOL_CALL_NEEDED:
                    # Extract tool call information
                    tool_id = ctypes.string_at(result.tool_call.id).decode('utf-8')
                    tool_name = ctypes.string_at(result.tool_call.tool_name).decode('utf-8')
                    args_json = ctypes.string_at(result.tool_call.arguments_json).decode('utf-8')
                    
                    print(f"\nTool call needed: {tool_name}")
                    print(f"Arguments: {args_json}")
                    
                    # Parse arguments
                    args = json.loads(args_json)
                    
                    # Execute the tool
                    tool_result = execute_tool(tool_name, args)
                    print(f"Tool result: {tool_result}")
                    
                    # Provide the tool result back to the agent
                    reply_state = goose.goose_agent_reply_tool_result(
                        reply_state,
                        tool_id.encode('utf-8'),
                        tool_result.encode('utf-8')
                    )
                    
                    if not reply_state:
                        goose.goose_free_string(result.tool_call.id)
                        goose.goose_free_string(result.tool_call.tool_name)
                        goose.goose_free_string(result.tool_call.arguments_json)
                        return "Error providing tool result"
                        
                    # Free tool call resources
                    goose.goose_free_string(result.tool_call.id)
                    goose.goose_free_string(result.tool_call.tool_name)
                    goose.goose_free_string(result.tool_call.arguments_json)
                
            return response_text
            
        finally:
            # Free reply state
            goose.goose_agent_reply_state_free(reply_state)

def main():
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        return

    # Create agent with ephemeral config
    agent = GooseAgent(api_key=api_key, model_name="databricks-dbrx-instruct", host=host, ephemeral=True)

    print("\n=== Non-streaming API with Tool Support ===")
    print("Type a message (or 'quit' to exit):")
    while True:
        user_input = input("> ")
        if user_input.lower() in ("quit", "exit"):
            break
            
        print("Processing with tool support...")
        reply = agent.send_message_non_streaming(user_input)
        print(f"\nAgent: {reply}\n")

if __name__ == "__main__":
    main()