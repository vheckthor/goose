"""
Helper functions for safely handling FFI resources in the Goose example

These functions handle proper conversion between raw C pointers (c_void_p)
and Python data types, ensuring memory is properly managed.
"""

import ctypes
import json

def get_message_content(result, goose_free_fn):
    """
    Safely extract content from a message and free the memory
    
    This function handles conversion from the raw C pointer to Python string,
    frees the memory and extracts the content from the JSON response.
    """
    if not result.message or result.message == 0:
        return "Empty response"

    try:
        # Get a copy of the message - note we're using the raw pointer value
        # which is safe because we're working with c_void_p type
        msg_data = ctypes.string_at(result.message).decode('utf-8')
        
        # Free the memory - use the raw pointer directly
        goose_free_fn(result.message)
        
        # Clear the pointer to prevent double-free
        # This is safe because we're using c_void_p in the struct definition
        result.message = 0
        
        try:
            # Parse as JSON
            msg_obj = json.loads(msg_data)
            
            # Extract text content if available
            text_parts = []
            for content in msg_obj.get("content", []):
                if content.get("type") == "text":
                    text_parts.append(content.get("text", ""))
                    
            if text_parts:
                return "\n".join(text_parts)
            elif msg_data.startswith('"') and msg_data.endswith('"'):
                # Sometimes the message is just a simple string
                return msg_data.strip('"')
            else:
                # If no text content, return the raw JSON
                return msg_data
                
        except json.JSONDecodeError:
            # If not valid JSON, return as-is
            return msg_data
            
    except Exception as e:
        # If any error occurs, return the error message
        return f"Error processing message: {e}"

def extract_tool_call(tool_call, goose_free_fn):
    """
    Safely extract data from a tool call and free the memory
    
    Since we're using c_void_p in the struct definition, we can safely
    handle the raw pointers and free them correctly.
    """
    tool_id = None
    tool_name = None
    args_json = None
    
    try:
        # Extract tool ID
        if tool_call.id and tool_call.id != 0:
            tool_id = ctypes.string_at(tool_call.id).decode('utf-8')
            goose_free_fn(tool_call.id)
            tool_call.id = 0
            
        # Extract tool name
        if tool_call.tool_name and tool_call.tool_name != 0:
            tool_name = ctypes.string_at(tool_call.tool_name).decode('utf-8')
            goose_free_fn(tool_call.tool_name)
            tool_call.tool_name = 0
            
        # Extract arguments
        if tool_call.arguments_json and tool_call.arguments_json != 0:
            args_json = ctypes.string_at(tool_call.arguments_json).decode('utf-8')
            goose_free_fn(tool_call.arguments_json)
            tool_call.arguments_json = 0
            
        # Parse arguments if available
        args = {}
        if args_json:
            try:
                args = json.loads(args_json)
            except json.JSONDecodeError:
                print(f"Warning: Could not parse arguments JSON: {args_json}")
                
        return tool_id, tool_name, args
        
    except Exception as e:
        print(f"Error extracting tool call: {e}")
        # Free any remaining resources
        if tool_call.id and tool_call.id != 0:
            goose_free_fn(tool_call.id)
            tool_call.id = 0
        if tool_call.tool_name and tool_call.tool_name != 0:
            goose_free_fn(tool_call.tool_name)
            tool_call.tool_name = 0
        if tool_call.arguments_json and tool_call.arguments_json != 0:
            goose_free_fn(tool_call.arguments_json)
            tool_call.arguments_json = 0
        return None, None, {}