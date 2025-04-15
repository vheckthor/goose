#!/usr/bin/env python3
"""
Simple Ollama Model Tester

Tests Ollama models with a system prompt and user query,
saving just the text responses.

Usage:
  ./ollama_tester.py [--models MODEL1,MODEL2,...] [--prompt PROMPT_FILE] [--query "Your query"]
"""

import argparse
import json
import os
import requests
import subprocess
import sys

# ===== EDIT THIS LIST TO CHANGE THE DEFAULT MODELS TO TEST =====
MODELS_TO_TEST = [
    "gemma3:12b",
    "deepseek-r1:7b",
    "deepseek-r1:14b"
]
# =============================================================

def show_available_models():
    """Show available models using the ollama list command"""
    try:
        result = subprocess.run(['ollama', 'list'], capture_output=True, text=True)
        print(result.stdout)
    except Exception as e:
        print(f"Error running 'ollama list': {e}")

def test_model(model_name, system_prompt, user_query, output_dir="."):
    """Test a specific model with the given system prompt and user query"""
    print(f"Testing model: {model_name}")
    
    # Create the API request
    payload = {
        "model": model_name,
        "system": system_prompt,
        "prompt": user_query,
        "stream": False
    }
    
    # Generate safe filename
    safe_filename = model_name.replace(':', '_').replace('/', '_')
    
    try:
        # Make the API call
        response = requests.post("http://localhost:11434/api/generate", json=payload)
        
        # Extract and save just the response text
        try:
            response_data = response.json()
            response_text = response_data.get('response', 'No response found')
            txt_path = os.path.join(output_dir, f"response_{safe_filename}.txt")
            with open(txt_path, 'w') as f:
                f.write(response_text)
            print(f"Response saved to {txt_path}")
        except json.JSONDecodeError:
            txt_path = os.path.join(output_dir, f"response_{safe_filename}.txt")
            with open(txt_path, 'w') as f:
                f.write(f"Error: Could not parse JSON response from Ollama API.")
            print(f"Error parsing response - see {txt_path}")
    except Exception as e:
        print(f"Error testing model {model_name}: {e}")
    
    print("----------------------------------------")

def main():
    parser = argparse.ArgumentParser(description="Test Ollama models with a system prompt and user query")
    parser.add_argument("--models", help="Comma-separated list of models to test")
    parser.add_argument("--prompt", default="prompt.txt", help="Path to system prompt file")
    parser.add_argument("--query", default="list files in this directory", help="User query")
    parser.add_argument("--output", default=".", help="Output directory for responses")
    parser.add_argument("--all", action="store_true", help="Test all available models")
    
    args = parser.parse_args()
    
    # Show available models
    print("Available models:")
    show_available_models()
    
    # Read the system prompt
    try:
        with open(args.prompt, 'r') as f:
            system_prompt = f.read()
    except Exception as e:
        print(f"Error reading prompt file: {e}")
        sys.exit(1)
    
    # Get models to test
    if args.models:
        models = args.models.split(',')
    elif args.all:
        try:
            response = requests.get("http://localhost:11434/api/tags")
            if response.status_code == 200:
                models_data = response.json()
                models = [model["name"] for model in models_data.get("models", [])]
                if not models:
                    print("No models found.")
                    sys.exit(1)
            else:
                print(f"Error fetching models: {response.status_code}")
                sys.exit(1)
        except Exception as e:
            print(f"Error connecting to Ollama API: {e}")
            sys.exit(1)
    else:
        models = MODELS_TO_TEST
    
    # Create output directory if it doesn't exist
    os.makedirs(args.output, exist_ok=True)
    
    # Test each model
    for model in models:
        test_model(model, system_prompt, args.query, args.output)

if __name__ == "__main__":
    main()