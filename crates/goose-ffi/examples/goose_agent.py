#!/usr/bin/env python3
"""
Python example for using the Goose FFI interface.

This example demonstrates how to:
1. Load the Goose FFI library
2. Create an agent with a provider
3. Register tools with the agent
4. Use the non-streaming API to send messages, handle tool calls, and get responses
"""

import ctypes
import json
import os
import platform
import time
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER
from enum import IntEnum
import sys
import traceback
from handle_ffi import get_message_content, extract_tool_call

# Simple .env file loader
def load_env_file(path='.env'):
    """Load environment variables from .env file"""
    if os.path.exists(path):
        print(f"Loading environment from {path}")
        with open(path, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    key, value = line.split('=', 1)
                    os.environ[key.strip()] = value.strip().strip('"\'')
    else:
        print(f"Warning: {path} file not found")

# Load environment variables from .env file
load_env_file()

# Platform-specific dynamic lib name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
elif platform.system() == "Windows":
    LIB_NAME = "goose_ffi.dll"
else:
    raise RuntimeError("Unsupported platform")

# Try to find the library path
if os.path.exists(os.path.join(os.path.dirname(__file__), LIB_NAME)):
    # Check current directory first (for build_local.sh)
    LIB_PATH = os.path.join(os.path.dirname(__file__), LIB_NAME)
elif os.path.exists(os.path.join(os.path.dirname(__file__), "../../..", "target", "release", LIB_NAME)):
    # Check release build directory
    LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "release", LIB_NAME)
elif os.path.exists(os.path.join(os.path.dirname(__file__), "../../..", "target", "debug", LIB_NAME)):
    # Check debug build directory
    LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "debug", LIB_NAME)
elif os.path.exists(os.path.join("/app", LIB_NAME)):
    # Docker container case
    LIB_PATH = os.path.join("/app", LIB_NAME)
else:
    print("Error: Could not find the Goose FFI library. Make sure to build it first.")
    print("Try running: cargo build -p goose-ffi --release")
    sys.exit(1)

print(f"Loading library from: {LIB_PATH}")

# Load library
try:
    goose = ctypes.CDLL(LIB_PATH)
    print("Successfully loaded Goose FFI library")
except Exception as e:
    print(f"Error loading library: {e}")
    sys.exit(1)

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
        ("id", c_void_p),
        ("tool_name", c_void_p),
        ("arguments_json", c_void_p),
    ]
    
class ReplyStepResult(Structure):
    _fields_ = [
        ("status", c_uint32),
        ("message", c_void_p),
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

# Function signatures - Tool registration API
goose.goose_agent_register_tools.argtypes = [goose_AgentPtr, c_char_p, c_char_p, c_char_p]
goose.goose_agent_register_tools.restype = c_bool

# Function signatures - Non-streaming API
goose.goose_agent_reply_begin.argtypes = [goose_AgentPtr, c_char_p]
goose.goose_agent_reply_begin.restype = c_void_p

goose.goose_agent_reply_step.argtypes = [c_void_p]
goose.goose_agent_reply_step.restype = ReplyStepResult

goose.goose_agent_reply_tool_result.argtypes = [c_void_p, c_char_p, c_char_p]
goose.goose_agent_reply_tool_result.restype = c_void_p

goose.goose_agent_reply_state_free.argtypes = [c_void_p]
goose.goose_agent_reply_state_free.restype = None

# Function signature for new non-yielding API
goose.goose_agent_reply_non_yielding.argtypes = [goose_AgentPtr, c_char_p, c_char_p, c_char_p]
goose.goose_agent_reply_non_yielding.restype = c_void_p

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
            expression = args.get("expression", "0")
            print(f"Calculating expression: {expression}")
            # Safe evaluation of the expression
            result = eval(str(expression), {"__builtins__": {}}, {"abs": abs, "round": round})
            return f"The result is: {result}"
        except Exception as e:
            print(f"Error in calculator: {e}")
            return f"Error calculating: {e}"
    elif tool_name == "weather":
        location = args.get("location", "unknown")
        print(f"Getting weather for: {location}")
        return f"Weather in {location} is currently sunny, 72°F."
    else:
        print(f"Unknown tool requested: {tool_name}")
        return f"Unknown tool: {tool_name}"

def should_force_tool_use(message):
    """Check if we should force tool use based on the message content"""
    message = message.lower()
    
    # Calculator patterns
    calc_patterns = [
        # Simple arithmetic
        r'\d+\s*[\+\-\*\/]\s*\d+',
        # Common calculation words
        r'calculate',
        r'compute',
        r'what is \d',
        r'what\'s \d',
        r'solve',
        r'evaluate'
    ]
    
    # Weather patterns
    weather_patterns = [
        r'weather in',
        r'what\'s the weather',
        r'what is the weather',
        r'temperature in',
        r'is it (rain|snow|sunny|cloud)',
        r'forecast'
    ]
    
    import re
    
    # Check calculator patterns
    for pattern in calc_patterns:
        if re.search(pattern, message):
            return "calculator", True
            
    # Check weather patterns
    for pattern in weather_patterns:
        if re.search(pattern, message):
            return "weather", True
    
    return None, False

class GooseAgent:
    def __init__(self, provider_type=ProviderType.DATABRICKS, api_key=None, model_name=None, host=None, ephemeral=False, auto_execute_tools=False):
        print(f"Provider: {provider_type.name}")
        print(f"Host: {host}")
        print(f"API key: {'*' * 4 + api_key[-4:] if api_key else 'None'}")
        print(f"Ephemeral config: {ephemeral}")
        print(f"Auto execute tools: {auto_execute_tools}")
        
        self.config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key.encode("utf-8") if api_key else None,
            model_name=model_name.encode("utf-8") if model_name else None,
            host=host.encode("utf-8") if host else None,
            ephemeral=ephemeral,
        )
        
        # Flag to control automatic tool execution
        self.auto_execute_tools = auto_execute_tools
        
        try:
            self.agent = goose.goose_agent_new(ctypes.byref(self.config))
            if not self.agent:
                error_msg = "Failed to create Goose agent - agent pointer is null"
                print(f"ERROR: {error_msg}")
                raise RuntimeError(error_msg)
            print("Agent created successfully")
        except Exception as e:
            print(f"Exception creating agent: {e}")
            traceback.print_exc()
            raise RuntimeError(f"Failed to create Goose agent: {e}")
            
        # Define function-style tools with OpenAI-compatible format
        # This approach is more likely to be recognized by models trained on function calling
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

When you need to use a tool, respond with a tool request in your message content. The system will recognize tool requests and execute them appropriately.

IMPORTANT: When asked to calculate something, you MUST use the calculator tool. Do not calculate things yourself.
"""
        self.register_tools(extension_name="mc-test", instructions=instructions)

    def __del__(self):
        if getattr(self, "agent", None):
            goose.goose_agent_free(self.agent)
            
    def register_tools(self, tools=None, extension_name=None, instructions=None):
        """Register tools with the agent.
        
        Args:
            tools: List of tool definitions (defaults to self.tools if None)
            extension_name: Name for the extension (optional)
            instructions: Instructions for using the tools (optional)
        
        Returns:
            bool: True if tools were registered successfully, False otherwise
        """
        if tools is None:
            tools = self.tools
            
        # Convert tools to JSON string
        FRONTEND_CONFIG = {
                    "name": "pythonclient",
                    "type": "frontend",
                    "tools": tools,
                    "instructions": instructions,
        }
        tools_json = json.dumps(tools).encode("utf-8")
        print(f"DEBUG: Registering {len(tools)} tools")
        print(f"DEBUG: Tools JSON: {tools_json}...")
        
        # Convert extension_name and instructions to bytes if provided
        ext_name = extension_name.encode("utf-8") if extension_name else None
        ext_instructions = instructions.encode("utf-8") if instructions else None
        
        print(f"DEBUG: Extension name: {extension_name}")
        print(f"DEBUG: Instructions provided: {instructions is not None}")
        
        # Register tools with the agent
        try:
            success = goose.goose_agent_register_tools(
                self.agent, 
                tools_json, 
                ext_name, 
                ext_instructions
            )
            
            if not success:
                print("Warning: Failed to register tools with agent")
                return False
                
            print(f"DEBUG: Successfully registered {len(tools)} tools")
            return True
        except Exception as e:
            print(f"ERROR: Exception during tool registration: {e}")
            return False

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
    
    def send_message_non_streaming(self, message: str, is_retry=False) -> str:
        """Send a message using the non-streaming API with tool handling."""
        # For tool requests, we need to explicitly ask the model to use tools
        if not is_retry and any(keyword in message.lower() for keyword in ["calculate", "what is", "what's", "weather"]):
            # Enhance the message to explicitly request tool use
#             enhanced_msg = f"""Please use the appropriate tool to answer this question and in the response include tool calls and arguments: {message}
#
# Available tools:
# - calculator: Use this to perform mathematical calculations
# - weather: Use this to get weather information
#
# Please respond with a tool call."""
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
        print(f"DEBUG: Sending message: {message}")
        reply_state = goose.goose_agent_reply_begin(self.agent, msg)
        if not reply_state:
            return "Error starting reply"
            
        try:
            response_text = ""
            
            # Process steps until complete
            while True:
                result = goose.goose_agent_reply_step(reply_state)
                print(f"DEBUG: Step result status: {result.status}")
                
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
                    try:
                        # Extract tool call data
                        tool_id, tool_name, args = extract_tool_call(result.tool_call, goose.goose_free_string)
                        
                        if not tool_id or not tool_name:
                            print("Error: Missing tool ID or name")
                            response_text = "Error: Missing tool information"
                            break
                            
                        print(f"Tool: {tool_name}")
                        print(f"Arguments: {args}")
                        
                        # Execute the tool
                        tool_result = execute_tool(tool_name, args)
                        print(f"Tool result: {tool_result}")
                        
                        # Provide the tool result back to the agent
                        new_state = goose.goose_agent_reply_tool_result(
                            reply_state,
                            tool_id.encode('utf-8'),
                            tool_result.encode('utf-8')
                        )
                        
                        if not new_state:
                            print("Error providing tool result to agent")
                            response_text = "Error: Failed to process tool result"
                            break
                            
                    except Exception as e:
                        print(f"Error processing tool call: {e}")
                        traceback.print_exc()
                        response_text = f"Error processing tool call: {str(e)}"
                        break
                
            return response_text
            
        finally:
            # Free reply state
            if reply_state:
                goose.goose_agent_reply_state_free(reply_state)
                
    def send_message_non_yielding(self, message: str, tool_responses=None) -> str:
        """Send a message using the fully non-yielding API.
        
        This implementation uses the new goose_agent_reply_non_yielding FFI function
        which handles the entire conversation, including tool calls, in one step.
        
        Args:
            message: User message to send
            tool_responses: Optional list of tuples (id, result) for tool responses
            
        Returns:
            String response from the agent
        """
        # Create a message object with the required created field
        import time
        current_time = int(time.time())
        
        messages = [{
            "role": "user",
            "created": current_time,
            "content": [{"type": "text", "text": message}]
        }]
        
        # Convert to JSON
        messages_json = json.dumps(messages).encode("utf-8")
        
        # Empty tool requests and responses if not provided
        tool_requests_json = "[]".encode("utf-8")
        tool_responses_json = "[]".encode("utf-8")
        
        # Include tool responses if provided
        if tool_responses:
            tool_responses_json = json.dumps(tool_responses).encode("utf-8")
        
        print(f"DEBUG: Sending non-yielding message: {message}")
        print(f"DEBUG: With tool responses: {tool_responses}")
        
        # Call the non-yielding API
        response_ptr = goose.goose_agent_reply_non_yielding(
            self.agent,
            messages_json,
            tool_requests_json,
            tool_responses_json
        )
        
        if not response_ptr:
            return "Error: NULL response from agent"
            
        try:
            # Convert response to string
            response_str = ctypes.string_at(response_ptr).decode("utf-8")
            print(f"DEBUG: Got raw response: {response_str}")
            
            # Parse response JSON
            try:
                response_obj = json.loads(response_str)
                
                # Extract text content and check for tool requests
                text_parts = []
                tool_requests = []
                
                for content in response_obj.get("content", []):
                    if content.get("type") == "text":
                        text_parts.append(content.get("text", ""))
                    elif content.get("type") == "toolRequest":  # Note the casing difference
                        # Found a tool request
                        tool_requests.append(content)
                
                # Build response text
                response_text = "\n".join(text_parts) if text_parts else ""
                
                # If we found tool requests, add them to the response
                if tool_requests:
                    # For debugging, let's include that we found tool requests
                    print(f"\nFound {len(tool_requests)} tool requests in response")
                    for i, req in enumerate(tool_requests):
                        tool_id = req.get("id")
                        tool_call = req.get("toolCall", {}).get("value", {})
                        tool_name = tool_call.get("name")
                        tool_args = tool_call.get("arguments", {})
                        print(f"Tool request {i}: {tool_name} (ID: {tool_id})")
                        print(f"Arguments: {tool_args}")
                        
                        # Store the tool ID and name in a property of this instance so we can 
                        # access it in future calls
                        if not hasattr(self, "last_tool_request"):
                            self.last_tool_request = {}
                        self.last_tool_request[tool_name] = tool_id
                    
                    # Optionally auto-execute tools if the auto_execute_tools flag is set
                    if hasattr(self, "auto_execute_tools") and self.auto_execute_tools:
                        print(f"\nAuto-executing tool requests...")
                        
                        # Process all tool requests
                        tool_responses = []
                        for req in tool_requests:
                            tool_call = req.get("toolCall", {}).get("value", {})
                            tool_name = tool_call.get("name")
                            tool_id = req.get("id")
                            tool_args = tool_call.get("arguments", {})
                            
                            # Execute the tool
                            tool_result = execute_tool(tool_name, tool_args)
                            print(f"Tool {tool_name} result: {tool_result}")
                            
                            # Add to list of tool responses
                            tool_responses.append((tool_id, tool_result))
                        
                        # If we have tool responses, send them back to get final response
                        if tool_responses:
                            print("Sending tool responses back to agent...")
                            
                            # Convert to JSON
                            tool_responses_json = json.dumps(tool_responses).encode("utf-8")
                            
                            # Make the FFI call directly to avoid recursion issues
                            response_ptr = goose.goose_agent_reply_non_yielding(
                                self.agent,
                                json.dumps([{
                                    "role": "user", 
                                    "created": int(time.time()),
                                    "content": [{"type": "text", "text": message}]
                                }]).encode("utf-8"),
                                "[]".encode("utf-8"),  # Empty tool requests
                                tool_responses_json
                            )
                            
                            if not response_ptr:
                                return "Error: NULL response from agent"
                                
                            try:
                                # Convert response to string
                                final_response_str = ctypes.string_at(response_ptr).decode("utf-8")
                                print(f"DEBUG: Got final response: {final_response_str}")
                                
                                # Parse response JSON
                                try:
                                    final_response_obj = json.loads(final_response_str)
                                    
                                    # Extract text from response
                                    final_text_parts = []
                                    for content in final_response_obj.get("content", []):
                                        if content.get("type") == "text":
                                            final_text_parts.append(content.get("text", ""))
                                    
                                    final_response = "\n".join(final_text_parts) if final_text_parts else final_response_str
                                    return final_response
                                    
                                except json.JSONDecodeError:
                                    return final_response_str
                                    
                            finally:
                                # Free the response string
                                goose.goose_free_string(response_ptr)
                    
                    # This is a non-yielding API, so we return the response with the tool request
                    # The calling code is responsible for executing the tool and sending the result
                    # unless auto_execute_tools is enabled
                    return response_text
                
                # No tool requests, just return the text
                return response_text
                
            except json.JSONDecodeError:
                # If not valid JSON, return as-is
                return response_str
                
        finally:
            # Free the response string
            goose.goose_free_string(response_ptr)

def main():
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
        print("\n=== Goose FFI API Example ===")
        print("Available tools:")
        for tool in agent.tools:
            print(f"  - {tool['name']}: {tool['description']}\n")
            
        print("\nAvailable modes:")
        print("  1. Streaming (non-tool) - Use a streaming response without tool support")
        print("  2. Non-streaming with tools - Use step-by-step tool processing (yielding)")
        print("  3. Non-yielding with tools - Use single-call tool processing (non-yielding)")
        print("\nType mode:message (or 'quit' to exit):")
        
        while True:
            user_input = input("\n> ")
            if user_input.lower() in ("quit", "exit"):
                break
                
            # Parse mode prefix
            parts = user_input.split(":", 1)
            mode = "2"  # Default to non-streaming with tools
            if len(parts) > 1 and parts[0] in ("1", "2", "3"):
                mode = parts[0]
                message = parts[1].strip()
            else:
                message = user_input
                
            print(f"Processing in mode {mode}...")
            try:
                if mode == "1":
                    # Streaming mode (no tools)
                    reply = agent.send_message(message)
                elif mode == "3":
                    # Non-yielding mode
                    reply = agent.send_message_non_yielding(message)
                else:
                    # Default: non-streaming with tools
                    reply = agent.send_message_non_streaming(message)
                    
                print(f"\nAgent: {reply}\n")
            except Exception as e:
                print(f"\nError processing message: {e}")
                traceback.print_exc()
                
    except Exception as e:
        print(f"Failed to initialize agent: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()
