#!/usr/bin/env python3
"""
Test script for the fixed Goose FFI implementation
"""

import os
import sys
import traceback

# Set environment variables for testing
os.environ["DATABRICKS_API_KEY"] = "dummy_key_for_testing"
os.environ["DATABRICKS_HOST"] = "https://dummy-host.databricks.com"

# Import the fixed agent
import goose_agent_fixed

try:
    print("Starting test...")
    
    # Create agent
    agent = goose_agent_fixed.GooseAgent(
        api_key=os.getenv("DATABRICKS_API_KEY"),
        model_name="databricks-meta-llama-3-1-70b-instruct",
        host=os.getenv("DATABRICKS_HOST"),
        ephemeral=True
    )
    
    print("Agent created successfully")
    
    # Set up default tools
    agent.setup_default_tools()
    print("Tools set up successfully")
    
    # Test a simple calculation
    response = agent.send_message_non_streaming("What is 5 + 3?")
    print(f"Response: {response}")
    
except Exception as e:
    print(f"Error: {e}")
    traceback.print_exc()