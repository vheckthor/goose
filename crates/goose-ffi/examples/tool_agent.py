#!/usr/bin/env python3
"""
Python Tool Agent Example

This example demonstrates how to use the Goose FFI interface to create an
agent that can invoke tools.
"""

import ctypes
import json
import os
import platform
from ctypes import c_char_p, c_void_p, c_bool, c_uint32, Structure, POINTER, CFUNCTYPE, c_size_t
from enum import IntEnum
from typing import Dict, Any, Optional, List, Callable

# Determine the platform-specific library name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
elif platform.system() == "Windows":
    LIB_NAME = "goose_ffi.dll"
else:
    raise RuntimeError(f"Unsupported platform: {platform.system()}")

# Find the library path relative to this script
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
LIB_PATH = os.path.join(SCRIPT_DIR, "..", "target", "debug", LIB_NAME)

# Load the FFI library
try:
    goose_lib = ctypes.CDLL(LIB_PATH)
except OSError as e:
    print(f"Error loading library: {e}")
    print(f"Make sure you've built the library with 'cargo build' and the path is correct: {LIB_PATH}")
    exit(1)

# Provider type enum values
class ProviderType(IntEnum):
    DATABRICKS = 0  # Databricks AI provider

# Tool parameter type enum values
class ToolParamType(IntEnum):
    STRING = 0   # String parameter type
    NUMBER = 1   # Number parameter type
    BOOLEAN = 2  # Boolean parameter type
    ARRAY = 3    # Array parameter type
    OBJECT = 4   # Object parameter type

# Tool parameter requirement enum values
class ToolParamRequirement(IntEnum):
    REQUIRED = 0  # Parameter is required
    OPTIONAL = 1  # Parameter is optional

# Define C-compatible structures and callbacks
class ProviderConfig(Structure):
    _fields_ = [
        ("provider_type", c_uint32),  # 0 = Databricks (currently only supported provider)
        ("api_key", c_char_p),
        ("model_name", c_char_p),
        ("host", c_char_p),
    ]

class ToolParamDef(Structure):
    _fields_ = [
        ("name", c_char_p),
        ("description", c_char_p),
        ("param_type", c_uint32),
        ("required", c_uint32),
    ]

class ToolParam(Structure):
    _fields_ = [
        ("name", c_char_p),
        ("value", c_char_p),
    ]

# Tool callback type
ToolCallbackFn = CFUNCTYPE(c_char_p, c_size_t, POINTER(ToolParam), c_void_p)

# Set up the function signatures
goose_lib.goose_agent_new.argtypes = [POINTER(ProviderConfig)]
goose_lib.goose_agent_new.restype = c_void_p

goose_lib.goose_agent_free.argtypes = [c_void_p]
goose_lib.goose_agent_free.restype = None

goose_lib.goose_agent_send_message.argtypes = [c_void_p, c_char_p]
goose_lib.goose_agent_send_message.restype = c_char_p

goose_lib.goose_free_string.argtypes = [c_char_p]
goose_lib.goose_free_string.restype = None

goose_lib.goose_create_tool_schema.argtypes = [c_char_p, c_char_p, POINTER(ToolParamDef), c_size_t]
goose_lib.goose_create_tool_schema.restype = c_char_p

goose_lib.goose_agent_register_tool_callback.argtypes = [c_void_p, c_char_p, c_char_p, c_char_p, ToolCallbackFn, c_void_p]
goose_lib.goose_agent_register_tool_callback.restype = c_bool

class GooseAgent:
    """Python wrapper for Goose Agent with tool support."""
    
    def __init__(self, provider_type: int = 0, api_key: Optional[str] = None, 
                 model_name: Optional[str] = None, host: Optional[str] = None):
        """
        Create a new Goose Agent.
        
        Args:
            provider_type: Provider type (0 = Databricks)
            api_key: Provider API key (or None to use environment variables)
            model_name: Model name (or None to use provider default)
            host: Provider host URL (or None to use environment variable)
        """
        # Convert strings to bytes for C compatibility
        api_key_bytes = api_key.encode('utf-8') if api_key else None
        model_name_bytes = model_name.encode('utf-8') if model_name else None
        host_bytes = host.encode('utf-8') if host else None
        
        # Create provider config
        config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key_bytes,
            model_name=model_name_bytes,
            host=host_bytes
        )
        
        # Create agent
        self.agent_ptr = goose_lib.goose_agent_new(ctypes.byref(config))
        if not self.agent_ptr:
            raise RuntimeError("Failed to create agent")
        
        # Store registered tool callbacks
        self._tool_callbacks = {}
    
    def __del__(self):
        """Cleanup agent when object is destroyed."""
        if hasattr(self, 'agent_ptr') and self.agent_ptr:
            goose_lib.goose_agent_free(self.agent_ptr)
    
    def create_tool_schema(self, name: str, description: str, parameters: List[Dict[str, Any]]) -> str:
        """
        Create a tool schema for tool registration.
        
        Args:
            name: Tool name
            description: Tool description
            parameters: List of parameter definitions. Each parameter is a dict with:
                - name: Parameter name
                - description: Parameter description
                - type: Parameter type (string, number, boolean, array, object)
                - required: Whether the parameter is required
        
        Returns:
            JSON schema string
        """
        # Convert parameters to C structures
        param_defs = []
        for param in parameters:
            param_type = ToolParamType.STRING  # Default
            if param.get('type') == 'string':
                param_type = ToolParamType.STRING
            elif param.get('type') == 'number':
                param_type = ToolParamType.NUMBER
            elif param.get('type') == 'boolean':
                param_type = ToolParamType.BOOLEAN
            elif param.get('type') == 'array':
                param_type = ToolParamType.ARRAY
            elif param.get('type') == 'object':
                param_type = ToolParamType.OBJECT
            
            required = ToolParamRequirement.REQUIRED if param.get('required', True) else ToolParamRequirement.OPTIONAL
            
            param_defs.append(ToolParamDef(
                name=param['name'].encode('utf-8'),
                description=param['description'].encode('utf-8'),
                param_type=param_type,
                required=required
            ))
        
        # Create array of param defs
        if param_defs:
            param_array_type = ToolParamDef * len(param_defs)
            param_array = param_array_type(*param_defs)
            param_ptr = ctypes.cast(param_array, POINTER(ToolParamDef))
            param_count = len(param_defs)
        else:
            param_ptr = None
            param_count = 0
        
        # Call the C function
        schema_ptr = goose_lib.goose_create_tool_schema(
            name.encode('utf-8'),
            description.encode('utf-8'),
            param_ptr,
            param_count
        )
        
        if not schema_ptr:
            raise RuntimeError("Failed to create tool schema")
        
        # Convert result to Python string and free the C string
        schema_str = ctypes.string_at(schema_ptr).decode('utf-8')
        goose_lib.goose_free_string(schema_ptr)
        
        return schema_str
    
    def register_tool(self, name: str, description: str, parameters: List[Dict[str, Any]],
                     callback: Callable[[Dict[str, Any]], Dict[str, Any]]):
        """
        Register a tool with the agent.
        
        Args:
            name: Tool name
            description: Tool description
            parameters: List of parameter definitions. Each parameter is a dict with:
                - name: Parameter name
                - description: Parameter description
                - type: Parameter type (string, number, boolean, array, object)
                - required: Whether the parameter is required
            callback: Python function to call when the tool is invoked
        """
        # Create schema
        schema_str = self.create_tool_schema(name, description, parameters)
        
        # Store the Python callback
        self._tool_callbacks[name] = callback
        
        # Create C callback wrapper
        @ToolCallbackFn
        def tool_callback_wrapper(param_count, params, user_data):
            # Extract parameters
            args = {}
            for i in range(param_count):
                param = params[i]
                param_name = ctypes.string_at(param.name).decode('utf-8')
                param_value_str = ctypes.string_at(param.value).decode('utf-8')
                
                # Parse JSON value
                try:
                    param_value = json.loads(param_value_str)
                    args[param_name] = param_value
                except json.JSONDecodeError:
                    args[param_name] = param_value_str
            
            # Call Python callback
            try:
                result = callback(args)
                result_json = json.dumps(result).encode('utf-8')
                return ctypes.create_string_buffer(result_json).raw
            except Exception as e:
                error_json = json.dumps({"error": str(e)}).encode('utf-8')
                return ctypes.create_string_buffer(error_json).raw
        
        # Keep a reference to prevent garbage collection
        self._tool_callbacks[f"{name}_c_wrapper"] = tool_callback_wrapper
        
        # Register with FFI
        success = goose_lib.goose_agent_register_tool_callback(
            self.agent_ptr,
            name.encode('utf-8'),
            description.encode('utf-8'),
            schema_str.encode('utf-8'),
            tool_callback_wrapper,
            None  # No user data
        )
        
        if not success:
            raise RuntimeError(f"Failed to register tool: {name}")
    
    def send_message(self, message: str) -> str:
        """
        Send a message to the agent.
        
        Args:
            message: The message to send
        
        Returns:
            The agent's response
        """
        # Send the message
        response_ptr = goose_lib.goose_agent_send_message(
            self.agent_ptr,
            message.encode('utf-8')
        )
        
        if not response_ptr:
            return "Error getting response from agent"
        
        # Read the response string and free it
        response = ctypes.string_at(response_ptr).decode('utf-8')
        goose_lib.goose_free_string(response_ptr)
        
        return response


def calculator_callback(params: Dict[str, Any]) -> Dict[str, Any]:
    """Calculator tool callback that performs arithmetic operations."""
    try:
        a = float(params.get('a', 0))
        b = float(params.get('b', 0))
        operation = params.get('operation', '')
        
        if operation == 'add':
            return {"result": a + b}
        elif operation == 'subtract':
            return {"result": a - b}
        elif operation == 'multiply':
            return {"result": a * b}
        elif operation == 'divide':
            if b == 0:
                return {"error": "Division by zero"}
            return {"result": a / b}
        else:
            return {"error": f"Unknown operation: {operation}"}
    except Exception as e:
        return {"error": str(e)}


def main():
    # Get API key and host URL from environment or let the library use them
    api_key = os.environ.get("DATABRICKS_API_KEY")
    host_url = os.environ.get("DATABRICKS_HOST")
    
    # Create agent with Databricks provider
    print("Creating Databricks agent...")
    agent = GooseAgent(
        provider_type=ProviderType.DATABRICKS,
        api_key=api_key,
        model_name="databricks-bge-large-en",
        host=host_url
    )
    
    # Register calculator tool
    print("Registering calculator tool...")
    agent.register_tool(
        name="calculator",
        description="Perform arithmetic operations on two numbers",
        parameters=[
            {
                "name": "a",
                "description": "First number",
                "type": "number",
                "required": True
            },
            {
                "name": "b",
                "description": "Second number",
                "type": "number",
                "required": True
            },
            {
                "name": "operation",
                "description": "Operation to perform: add, subtract, multiply, or divide",
                "type": "string",
                "required": True
            }
        ],
        callback=calculator_callback
    )
    
    # Prompt the user to provide instructions to the agent
    print("\nYou can now ask the agent to perform calculations.")
    print("Examples:")
    print("- Calculate 5 + 3")
    print("- What is 10 divided by 2?")
    print("- Multiply 7 by 6\n")
    
    # Interactive loop
    while True:
        try:
            user_input = input("> ")
            if user_input.lower() in ("quit", "exit"):
                break
            
            response = agent.send_message(user_input)
            print(f"Agent: {response}\n")
        except KeyboardInterrupt:
            break
        except Exception as e:
            print(f"Error: {e}\n")


if __name__ == "__main__":
    main()