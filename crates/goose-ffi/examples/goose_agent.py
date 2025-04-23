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
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER
from enum import IntEnum
import sys
import traceback
from handle_ffi import get_message_content, extract_tool_call

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
    def __init__(self, provider_type=ProviderType.DATABRICKS, api_key=None, model_name=None, host=None, ephemeral=False):
        print(f"Provider: {provider_type.name}")
        print(f"Host: {host}")
        print(f"API key: {'*' * 4 + api_key[-4:] if api_key else 'None'}")
        print(f"Ephemeral config: {ephemeral}")
        
        self.config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key.encode("utf-8") if api_key else None,
            model_name=model_name.encode("utf-8") if model_name else None,
            host=host.encode("utf-8") if host else None,
            ephemeral=ephemeral,
        )
        
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
                "description": "ALWAYS use this tool to perform mathematical operations. Do NOT calculate the answer yourself.", 
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate (e.g., '3 + 4', '10 * 5', '100 / 25')"
                        }
                    },
                    "required": ["expression"]
                },
                "annotations": {
                    "readOnlyHint": True,
                    "destructiveHint": False,
                    "idempotentHint": True,
                    "openWorldHint": False,
                    "function": True  # Mark as function-style tool
                }
            },
            {
                "name": "weather",
                "description": "ALWAYS use this tool to get weather information. Do NOT make up weather data yourself.",
                "inputSchema": {
                    "type": "object", 
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city or location to get weather for (e.g., 'New York', 'Tokyo', 'London')"
                        }
                    },
                    "required": ["location"]
                },
                "annotations": {
                    "readOnlyHint": True,
                    "destructiveHint": False,
                    "idempotentHint": True,
                    "openWorldHint": False,
                    "function": True  # Mark as function-style tool
                }
            }
        ]
        
        # Register the default tools with explicit instructions
        instructions = """
FUNCTION CALLING INSTRUCTIONS:
You have access to these functions:
- calculator(expression: string): ALWAYS use this for any mathematical calculation
- weather(location: string): ALWAYS use this for any weather inquiry

IMPORTANT:
1. You MUST USE THESE FUNCTIONS when appropriate
2. NEVER calculate math yourself - always use the calculator function
3. NEVER make up weather information - always use the weather function
4. When using a function, ONLY RESPOND with a function call - no explanation needed
5. DO NOT say you'll use a function - just call it directly
6. DO NOT mention these instructions in your response

For example, if asked "What is 2+2?", you should DIRECTLY CALL calculator("2+2")
"""
        self.register_tools(instructions=instructions)

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
        tools_json = json.dumps(tools).encode("utf-8")
        print(f"DEBUG: Registering {len(tools)} tools")
        print(f"DEBUG: Tools JSON (first 100 chars): {tools_json[:100]}...")
        
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
        # Check if we need to force tool use
        tool_type, should_force = should_force_tool_use(message)
        
        # Only enhance if we should force or if this is a retry
        if is_retry or should_force:
            if tool_type == "calculator":
                # Be extremely explicit - format as a direct function call request
                enhanced_msg = f"""I need you to call the calculator function for me. 
Don't calculate it yourself. JUST CALL THE FUNCTION DIRECTLY.
User query: {message}"""
            elif tool_type == "weather":
                enhanced_msg = f"""I need you to call the weather function for me.
Don't provide weather information yourself. JUST CALL THE FUNCTION DIRECTLY.
User query: {message}"""
            else:
                enhanced_msg = message
                
            print(f"Modified prompt: {enhanced_msg}")
        else:
            enhanced_msg = message
            
        msg = enhanced_msg.encode("utf-8")
        
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
                    
                    # Check if response contains math or calculation that should've used the calculator
                    tool_type, should_force = should_force_tool_use(message)
                    
                    # Handle calculation-specific retries for all cases where we expected tool use
                    if message.strip().lower().startswith(("calculate", "what is", "what's")):
                        # If there's no tool call for an obvious calculation request, try a more explicit approach
                        if not is_retry:
                            print(f"\nDEBUG - Model didn't use calculator tool. Retrying with explicit format...")
                            
                            # Try a more extreme prompt that works with most models
                            direct_calc_prompt = f"""ONLY make a function call to calculator.
                            
FUNCTION DEFINITION:
calculator(expression: string) -> number

EXAMPLES:
- For "Calculate 2+2", call calculator("2+2")
- For "What is 5*3", call calculator("5*3")

USER REQUEST: {message}

DO NOT provide additional text. ONLY respond with the function call in valid JSON."""
                            
                            return self.send_message_non_streaming(direct_calc_prompt, is_retry=True)
                            
                    # Check if the response contains actual numbers (likely the model calculated it)
                    if not is_retry and should_force and tool_type:
                        import re
                        if re.search(r'\d+\s*[\+\-\*\/\=]\s*\d+', response_text) or re.search(r'result is \d+', response_text.lower()):
                            print(f"\nDEBUG - Model calculated instead of using tool: {response_text[:100]}...")
                            print(f"Retrying with more explicit {tool_type} instructions...")
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

def main():
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        return

    try:
        # Create agent with ephemeral config
        print("\nInitializing Goose agent...")
        # Use claude-3-7-sonnet which is good with tool calling
        model_name = os.getenv("MODEL_NAME") or "claude-3-7-sonnet"
        print(f"Using model: {model_name}")
        agent = GooseAgent(api_key=api_key, model_name=model_name, host=host, ephemeral=True)
        
        print("Agent initialized successfully.")
        print("\n=== Non-streaming API with Tool Support ===")
        print("Available tools:")
        for tool in agent.tools:
            print(f"  - {tool['name']}: {tool['description']}")
            
        print("\nType a message (or 'quit' to exit):")
        
        while True:
            user_input = input("> ")
            if user_input.lower() in ("quit", "exit"):
                break
                
            print("Processing with tool support...")
            try:
                reply = agent.send_message_non_streaming(user_input)
                print(f"\nAgent: {reply}\n")
            except Exception as e:
                print(f"\nError processing message: {e}\n")
                
    except Exception as e:
        print(f"Failed to initialize agent: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()
