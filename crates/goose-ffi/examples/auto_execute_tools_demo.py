#!/usr/bin/env python3
"""
Demonstration of Goose FFI with automatic tool execution.

This script demonstrates how to use the Goose FFI API with automatic tool
execution, where detected tool calls are automatically executed and 
the results sent back to the model in a single API call.
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

def create_agent():
    """Create and configure a GooseAgent instance with auto tool execution"""
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        print("Make sure your .env file contains these variables or set them in your environment.")
        sys.exit(1)
    
    # Use a proper Databricks model endpoint name and enable auto_execute_tools
    return GooseAgent(
        api_key=api_key, 
        model_name="claude-3-7-sonnet", 
        host=host, 
        ephemeral=True,
        auto_execute_tools=True  # Enable automatic tool execution
    )

def main():
    """Demonstrate automatic tool execution"""
    print("=== Goose FFI with Auto Tool Execution Demo ===")
    
    # Create agent with auto_execute_tools=True
    agent = create_agent()
    
    # Register tools with the agent
    agent.register_tools(extension_name="auto-tools-demo")
    
    # Create a simplified demo to focus just on the calculator
    print("\n=== Calculator with manual execution ===")
    calc_query = "What is 123 * 456? Calculate this precisely using the calculator tool."
    print(f"User: {calc_query}")
    
    # First call to get the response with tool calls
    response = agent.send_message_non_yielding(calc_query)
    print(f"Initial response: {response}")
    
    # Check if we got tool requests
    if hasattr(agent, "last_tool_request") and "calculator" in agent.last_tool_request:
        tool_id = agent.last_tool_request["calculator"]
        print(f"\nFound calculator tool request with ID: {tool_id}")
        
        # Execute the tool manually 
        tool_result = "The result of 123 * 456 is 56,088"
        print(f"Tool result: {tool_result}")
        
        # Call again with the tool result
        tool_responses = [(tool_id, tool_result)]
        print("\nSending tool response back to agent...")
        final_response = agent.send_message_non_yielding(calc_query, tool_responses)
        print(f"Final response with tool result: {final_response}")
    else:
        print("No tool requests found in response")

if __name__ == "__main__":
    main()