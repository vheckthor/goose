#!/usr/bin/env python3
"""
Test script for the non-yielding Goose FFI function.

This script demonstrates how to use the non-yielding API directly from Python,
which simulates how a Kotlin service would interact with the FFI without yielding.
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

def test_simple_conversation():
    """Test a simple conversation without tool calls"""
    print("\n=== Testing simple conversation ===")
    agent = create_agent()
    
    message = "Tell me a short joke about programming."
    
    print(f"User: {message}")
    response = agent.send_message_non_yielding(message)
    print(f"Agent: {response}")

def test_calculator_tool():
    """Test calculator tool with the non-yielding API"""
    print("\n=== Testing calculator tool ===")
    agent = create_agent()
    
    # Use a clear calculator instruction to force a tool call
    message = "Please calculate exactly 123 * 456 using the calculator tool. I need the precise result."
    
    print(f"User: {message}")
    response = agent.send_message_non_yielding(message)
    print(f"Agent: {response}")
    
    # Check if we extracted tool requests
    if hasattr(agent, "last_tool_request") and agent.last_tool_request:
        print("\n✓ Successfully extracted tool requests from response!")
        for tool_name, tool_id in agent.last_tool_request.items():
            print(f"  Tool: {tool_name}, ID: {tool_id}")
        
        print("\nThis demonstrates that the non-yielding FFI function works correctly!")
        print("In a Kotlin implementation, you would:")
        print("1. Extract the tool calls from the response")
        print("2. Execute the tools in your client code")
        print("3. Send the results back to the agent")
        
    else:
        print("\n⚠ No tool requests found in the response.")

def test_weather_tool():
    """Test weather tool with the non-yielding API"""
    print("\n=== Testing weather tool ===")
    agent = create_agent()
    
    message = "Tell me the current weather in San Francisco, California. Please use the weather tool for this."
    
    print(f"User: {message}")
    response = agent.send_message_non_yielding(message)
    print(f"Agent: {response}")
    
    # Check the response to see if it contains a tool call
    if "I'll use the weather tool" in response or "weather tool" in response.lower():
        print("✓ Response mentions using the weather tool")
    
        # After the agent responds with intent to use the tool, we can simulate providing tool results 
        # Get the actual tool ID from the agent's stored last_tool_request
        if hasattr(agent, "last_tool_request") and "weather" in agent.last_tool_request:
            tool_id = agent.last_tool_request["weather"]
        else:
            # Fallback tool ID if we couldn't extract it
            tool_id = "toolu_bdrk_01WBYCh6u5JehWvFmnkxJYXY"
        tool_result = "Weather in San Francisco is currently sunny, 72°F with light winds from the west."
        
        print(f"\nProviding tool response with ID {tool_id}: {tool_result}")
        
        # Send the same message but with tool results this time
        tool_responses = [(tool_id, tool_result)]
        final_response = agent.send_message_non_yielding(message, tool_responses)
        print(f"Agent with tool results: {final_response}")
    else:
        print("⚠ Response does not mention using weather tool")

def test_multi_turn_conversation():
    """Test multiple messages in a conversation"""
    print("\n=== Testing multi-turn conversation ===")
    agent = create_agent()
    
    # First message
    message1 = "Hello! How are you today?"
    print(f"User: {message1}")
    response1 = agent.send_message_non_yielding(message1)
    print(f"Agent: {response1}")
    
    # Second message
    message2 = "Tell me about yourself."
    print(f"\nUser: {message2}")
    response2 = agent.send_message_non_yielding(message2)
    print(f"Agent: {response2}")

def create_agent():
    """Create and configure a GooseAgent instance"""
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    if not api_key or not host:
        print("Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set.")
        print("Make sure your .env file contains these variables or set them in your environment.")
        sys.exit(1)
    
    # Use a proper Databricks model endpoint name
    return GooseAgent(
        api_key=api_key, 
        model_name="claude-3-7-sonnet", 
        host=host, 
        ephemeral=True
    )

def main():
    """Run all the tests"""
    print("=== Non-yielding FFI API Test ===")
    
    # Run tests
    try:
        # Just run the calculator test
        test_calculator_tool()
        print("\nTest completed successfully!")
    except Exception as e:
        print(f"Error during testing: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()