#!/usr/bin/env python3
"""
Example of using the Goose FFI library from Python with ctypes.
This demonstrates both streaming and non-streaming APIs with tool support.
"""

import os
import ctypes
import json
import platform
import time
import sys
import traceback
from handle_ffi import get_message_content, extract_tool_call
from dotenv import load_dotenv

load_dotenv()

# Platform-specific dynamic lib name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
else:
    LIB_NAME = "goose_ffi.dll"

# Load the library
lib_path = os.path.join(os.path.dirname(__file__), LIB_NAME)
print(f"Loading library from: {lib_path}")
goose = ctypes.CDLL(lib_path)

# C types
class ProviderConfigFFI(ctypes.Structure):
    _fields_ = [
        ("provider_type", ctypes.c_uint32),
        ("api_key", ctypes.c_char_p),
        ("model_name", ctypes.c_char_p),
        ("host", ctypes.c_char_p),
        ("ephemeral", ctypes.c_bool),
    ]

class AsyncResult(ctypes.Structure):
    _fields_ = [
        ("succeeded", ctypes.c_bool),
        ("error_message", ctypes.c_char_p),
    ]

class ToolCallFFI(ctypes.Structure):
    _fields_ = [
        ("id", ctypes.c_char_p),
        ("tool_name", ctypes.c_char_p),
        ("arguments_json", ctypes.c_char_p),
    ]

class ReplyStepResult(ctypes.Structure):
    _fields_ = [
        ("status", ctypes.c_uint32),
        ("message", ctypes.c_char_p),
        ("tool_call", ToolCallFFI),
    ]

# Function signatures
goose.goose_agent_new.argtypes = [ctypes.POINTER(ProviderConfigFFI)]
goose.goose_agent_new.restype = ctypes.c_void_p

goose.goose_agent_free.argtypes = [ctypes.c_void_p]
goose.goose_agent_free.restype = None

goose.goose_agent_send_message.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
goose.goose_agent_send_message.restype = ctypes.c_char_p

goose.goose_agent_reply_begin.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
goose.goose_agent_reply_begin.restype = ctypes.c_void_p

goose.goose_agent_reply_step.argtypes = [ctypes.c_void_p]
goose.goose_agent_reply_step.restype = ReplyStepResult

goose.goose_agent_reply_tool_result.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p]
goose.goose_agent_reply_tool_result.restype = ctypes.c_void_p

goose.goose_agent_reply_state_free.argtypes = [ctypes.c_void_p]
goose.goose_agent_reply_state_free.restype = None

goose.goose_agent_register_tools.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
goose.goose_agent_register_tools.restype = ctypes.c_bool

goose.goose_free_string.argtypes = [ctypes.c_char_p]
goose.goose_free_string.restype = None

goose.goose_free_tool_call.argtypes = [ToolCallFFI]
goose.goose_free_tool_call.restype = None

# Provider types
class ProviderType:
    DATABRICKS = 0

# Reply status
class ReplyStatus:
    COMPLETE = 0
    TOOL_CALL_NEEDED = 1
    ERROR = 2

def c_char_to_string(c_char_p):
    """Safely convert C char pointer to Python string."""
    if c_char_p:
        return c_char_p.decode('utf-8')
    return None

class GooseAgent:
    def __init__(self, api_key=None, model_name=None, host=None, ephemeral=False):
        """Initialize a Goose agent with Databricks provider."""
        # Create config
        config = ProviderConfigFFI()
        config.provider_type = ProviderType.DATABRICKS
        config.api_key = api_key.encode('utf-8') if api_key else None
        config.model_name = model_name.encode('utf-8') if model_name else None
        config.host = host.encode('utf-8') if host else None
        config.ephemeral = ephemeral
        
        # Create agent
        self.agent = goose.goose_agent_new(ctypes.byref(config))
        if not self.agent:
            raise RuntimeError("Failed to create agent")
        
        self.tools = []
        self.tool_handlers = {}
    
    def __del__(self):
        """Clean up the agent when the object is destroyed."""
        if hasattr(self, 'agent') and self.agent:
            goose.goose_agent_free(self.agent)
    
    def register_tools(self, tools=None, extension_name=None, instructions=None):
        """Register tools with the agent."""
        if tools is None:
            tools = self.tools
        
        tools_json = json.dumps(tools).encode('utf-8')
        extension_name_bytes = extension_name.encode('utf-8') if extension_name else None
        instructions_bytes = instructions.encode('utf-8') if instructions else None
        
        print(f"DEBUG: Registering {len(tools)} tools")
        print(f"DEBUG: Tools JSON (first 100 chars): {tools_json[:100]}...")
        print(f"DEBUG: Extension name: {extension_name}")
        print(f"DEBUG: Instructions provided: {instructions is not None}")
        
        success = goose.goose_agent_register_tools(
            self.agent,
            tools_json,
            extension_name_bytes,
            instructions_bytes
        )
        
        if success:
            print(f"DEBUG: Successfully registered {len(tools)} tools")
        else:
            print(f"DEBUG: Failed to register tools")
        
        return success
    
    def add_tool(self, name, description, parameters, handler):
        """Add a tool to the agent."""
        tool = {
            "name": name,
            "description": description,
            "inputSchema": parameters
        }
        self.tools.append(tool)
        self.tool_handlers[name] = handler
    
    def setup_default_tools(self):
        """Set up default tools for testing."""
        # Calculator tool
        def calculator_handler(expression):
            try:
                # Use eval safely for simple math expressions
                allowed_chars = set('0123456789+-*/(). ')
                if not all(c in allowed_chars for c in expression):
                    return {"error": "Invalid characters in expression"}
                result = eval(expression)
                return {"result": result}
            except Exception as e:
                return {"error": str(e)}
        
        # Weather tool
        def weather_handler(location):
            # Mock weather data
            weather_data = {
                "New York": "Sunny, 72°F",
                "London": "Rainy, 55°F",
                "Tokyo": "Cloudy, 68°F",
                "default": f"Weather data not available for {location}"
            }
            return {"weather": weather_data.get(location, weather_data["default"])}
        
        # Add tools
        self.tools = [
            {
                "name": "calculator",
                "description": "Perform mathematical calculations. Use this for any arithmetic operations.", 
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate (e.g., '3 + 4', '10 * 5', '100 / 25')"
                        }
                    },
                    "required": ["expression"]
                }
            },
            {
                "name": "weather",
                "description": "Get weather information for a specific location.",
                "inputSchema": {
                    "type": "object", 
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city or location to get weather for (e.g., 'New York', 'Tokyo', 'London')"
                        }
                    },
                    "required": ["location"]
                }
            }
        ]
        
        # Register the default tools with explicit instructions
        instructions = """
You have access to the following tools:

1. calculator: Use this tool to perform mathematical calculations. When asked to calculate something, call this tool with the expression.
2. weather: Use this tool to get weather information for a location.

When you need to use a tool, respond with a tool call in the following format:
{
    "tool_calls": [
        {
            "id": "unique_id",
            "type": "function",
            "function": {
                "name": "tool_name",
                "arguments": "{\"param\": \"value\"}"
            }
        }
    ]
}

IMPORTANT: When asked to calculate something, you MUST use the calculator tool. Do not calculate things yourself.
"""
        self.register_tools(instructions=instructions)
        
        # Register handlers
        self.tool_handlers = {
            "calculator": calculator_handler,
            "weather": weather_handler
        }
    
    def send_message(self, message: str) -> str:
        """Send a message to the agent and get the response (streaming API)."""
        response_ptr = goose.goose_agent_send_message(self.agent, message.encode('utf-8'))
        
        if not response_ptr:
            return "Error: No response from agent"
        
        # Convert the C string to Python string
        response = ctypes.string_at(response_ptr).decode('utf-8')
        
        # Free the string using the proper C function provided by the library
        goose.goose_free_string(response_ptr)
        return response
    
    def send_message_non_streaming(self, message: str, is_retry=False) -> str:
        """Send a message using the non-streaming API with tool handling."""
        # For tool requests, we need to explicitly ask the model to use tools
        if not is_retry and any(keyword in message.lower() for keyword in ["calculate", "what is", "what's", "weather"]):
            # Enhance the message to explicitly request tool use
            enhanced_msg = f"""Please use the appropriate tool to answer this question: {message}

When you need to use a tool, respond with a tool call in the following format:
{{
    "tool_calls": [
        {{
            "id": "unique_id",
            "type": "function",
            "function": {{
                "name": "tool_name",
                "arguments": "{{\"param\": \"value\"}}"
            }}
        }}
    ]
}}

Available tools:
- calculator: Use this to perform mathematical calculations
- weather: Use this to get weather information

Please respond with a tool call."""
            msg = enhanced_msg.encode("utf-8")
        else:
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
                    # Handle error response
                    response_text = get_message_content(result, goose.goose_free_string)
                    if not response_text.startswith("Error:"):
                        response_text = f"Error: {response_text}"
                    return response_text
                    
                elif result.status == ReplyStatus.COMPLETE:
                    # Handle complete response
                    response_text = get_message_content(result, goose.goose_free_string)
                    
                    # For debugging, let's see what we got
                    print(f"DEBUG: Complete response: {response_text}")
                    
                    # Check if this is a 404 error
                    if "404 Not Found" in response_text and not is_retry:
                        print("DEBUG: Got 404 error, trying with a different model name...")
                        # Try with a different model name - use a more common Databricks model
                        self.model_name = "databricks-meta-llama-3-1-70b-instruct"
                        return self.send_message_non_streaming(message, is_retry=True)
                    
                    break
                    
                elif result.status == ReplyStatus.TOOL_CALL_NEEDED:
                    # Handle tool call
                    print("\nTool call needed")
                    tool_call_info = extract_tool_call(result, goose.goose_free_tool_call)
                    
                    if tool_call_info:
                        tool_id, tool_name, arguments_json = tool_call_info
                        print(f"Tool: {tool_name}")
                        print(f"Arguments: {arguments_json}")
                        
                        # Execute tool
                        if tool_name in self.tool_handlers:
                            try:
                                args = json.loads(arguments_json)
                                tool_result = self.tool_handlers[tool_name](**args)
                                result_json = json.dumps(tool_result)
                                print(f"Tool result: {result_json}")
                                
                                # Provide tool result
                                new_state = goose.goose_agent_reply_tool_result(
                                    reply_state,
                                    tool_id.encode('utf-8'),
                                    result_json.encode('utf-8')
                                )
                                
                                if not new_state:
                                    return "Error providing tool result"
                                
                                # Free old state and use new one
                                goose.goose_agent_reply_state_free(reply_state)
                                reply_state = new_state
                                
                            except Exception as e:
                                error_result = json.dumps({"error": str(e)})
                                new_state = goose.goose_agent_reply_tool_result(
                                    reply_state,
                                    tool_id.encode('utf-8'),
                                    error_result.encode('utf-8')
                                )
                                
                                if not new_state:
                                    return "Error providing tool error result"
                                
                                goose.goose_agent_reply_state_free(reply_state)
                                reply_state = new_state
                        else:
                            error_result = json.dumps({"error": f"Unknown tool: {tool_name}"})
                            new_state = goose.goose_agent_reply_tool_result(
                                reply_state,
                                tool_id.encode('utf-8'),
                                error_result.encode('utf-8')
                            )
                            
                            if not new_state:
                                return "Error providing unknown tool result"
                            
                            goose.goose_agent_reply_state_free(reply_state)
                            reply_state = new_state
                    else:
                        return "Error extracting tool call information"
                else:
                    return f"Unknown status: {result.status}"
            
            return response_text
            
        finally:
            if reply_state:
                goose.goose_agent_reply_state_free(reply_state)

def main():
    """Main function to demonstrate using the Goose FFI library."""
    print("Successfully loaded Goose FFI library")
    
    # Get configuration from environment
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        return

    try:
        # Create agent with ephemeral config
        print("\nInitializing Goose agent...")
        # Use a proper Databricks model endpoint name
        model_name = "claude-3-7-sonnet"
        print(f"Using model: {model_name}")
        agent = GooseAgent(api_key=api_key, model_name=model_name, host=host, ephemeral=True)
        
        print("Agent initialized successfully.")
        
        # Set up default tools
        agent.setup_default_tools()
        
        # Test non-streaming API with tool support
        print("\n=== Non-streaming API with Tool Support ===")
        print("Available tools:")
        for tool in agent.tools:
            print(f"  - {tool['name']}: {tool['description']}")
        
        print("\nType a message (or 'quit' to exit):")
        while True:
            user_input = input("> ")
            if user_input.lower() == 'quit':
                break
            
            print("Processing with tool support...")
            response = agent.send_message_non_streaming(user_input)
            print(f"\nAgent: {response}\n")
        
    except Exception as e:
        print(f"Failed to initialize agent: {e}")
        traceback.print_exc()

if __name__ == "__main__":
    main()
