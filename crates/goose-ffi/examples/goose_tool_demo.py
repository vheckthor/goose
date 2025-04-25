#!/usr/bin/env python3
"""
Simple demonstration of the Goose FFI implementation with tool detection.

This script demonstrates:
1. How to create a Goose agent via FFI
2. How to register tools
3. How to use the non-yielding function to get a response
4. How to extract tool calls from the response

Note: This demo focuses on tool extraction rather than full execution,
as different providers have different expectations for tool response formats.
"""

import os
import sys
import re
from goose_agent import GooseAgent

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

def run_simple_tool_demo():
    """Run a simple tool call extraction demo"""
    print("\n=== Goose FFI Tool Call Demo ===")
    
    # Create agent
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        sys.exit(1)
    
    # Create the agent
    agent = GooseAgent(
        api_key=api_key, 
        model_name="claude-3-7-sonnet", 
        host=host, 
        ephemeral=True
    )
    
    # Register tools
    agent.register_tools(extension_name="tool-demo")
    
    # Send a message asking for a calculation
    query = "What is 123 * 456? Calculate this precisely."
    print(f"\nUser: {query}")
    
    # Get response using non-yielding API
    response = agent.send_message_non_yielding(query)
    print(f"\nAgent: {response}")
    
    # Check if we got tool requests
    if hasattr(agent, "last_tool_request") and agent.last_tool_request:
        print("\n✅ Successfully extracted tool requests from response!")
        for tool_name, tool_id in agent.last_tool_request.items():
            print(f"  Tool: {tool_name}, ID: {tool_id}")
            
        print("\nThis demonstrates the key functionality needed for a Kotlin FFI implementation:")
        print("1. Send a message using the non-yielding FFI function")
        print("2. Extract tool calls from the response")
        print("3. Execute tools in the client")
        print("4. Send results back using the same non-yielding function")
        
    else:
        print("\n❌ No tool requests extracted from response")

if __name__ == "__main__":
    run_simple_tool_demo()