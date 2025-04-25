#!/usr/bin/env python3
"""
Example demonstrating how to use the goose-llm Rust library from Python.
"""

import goose_llm_py
import json

def main():
    # Create a calculator tool
    calculator_schema = {
        "type": "object",
        "required": ["operation", "numbers"],
        "properties": {
            "operation": {
                "type": "string",
                "enum": ["add", "subtract", "multiply", "divide"],
                "description": "The arithmetic operation to perform",
            },
            "numbers": {
                "type": "array",
                "items": {"type": "number"},
                "description": "List of numbers to operate on in order",
            }
        }
    }
    
    calculator_tool = goose_llm_py.create_tool(
        "calculator",
        "Perform basic arithmetic operations",
        calculator_schema
    )
    
    # Create a bash tool
    bash_schema = {
        "type": "object",
        "required": ["command"],
        "properties": {
            "command": {
                "type": "string",
                "description": "The shell command to execute"
            }
        }
    }
    
    bash_tool = goose_llm_py.create_tool(
        "bash_shell",
        "Run a shell command",
        bash_schema
    )
    
    tools = [calculator_tool, bash_tool]
    
    # Test with different prompts
    prompts = [
        "Add 10037 + 23123",
        "Write some random bad words to end of words.txt",
        "List all json files in the current directory and then multiply the count of the files by 7",
    ]
    
    for prompt in prompts:
        print(f"\n{'='*50}")
        print(f"User Input: {prompt}")
        print(f"{'='*50}")
        
        # Create a user message
        message = goose_llm_py.create_message("user", prompt)
        
        # Perform the completion
        response = goose_llm_py.perform_completion(
            provider="databricks",
            model_name="goose-claude-3-5-sonnet",
            system_preamble="You are a helpful assistant",
            messages=[message],
            tools=tools,
            check_tool_approval=True
        )
        
        # Print the response
        print(f"\nResponse role: {response.message.role}")
        print(f"Usage: input_tokens={response.usage.input_tokens}, "
              f"output_tokens={response.usage.output_tokens}, "
              f"total_tokens={response.usage.total_tokens}")
        
        # Print message content
        for content in response.message.content:
            if hasattr(content, 'text'):
                print(f"Text: {content.text}")
            elif hasattr(content, 'name'):
                print(f"Tool Request: {content.name} (id: {content.id})")
                print(f"Parameters: {content.parameters}")
        
        # Print tool approvals if available
        if response.tool_approvals:
            print(f"\nTool Approvals:")
            print(f"  Approved: {response.tool_approvals.approved}")
            print(f"  Needs Approval: {response.tool_approvals.needs_approval}")
            print(f"  Denied: {response.tool_approvals.denied}")

if __name__ == "__main__":
    main()