#!/bin/bash

# Function to run benchmark for an Ollama model
run_ollama_benchmark() {
    local model=$1
    echo "Starting Ollama with model: $model"
    
    # Set the environment variables
    export GOOSE_PROVIDER="ollama"
    export GOOSE_MODEL="$model"
    
    # Start ollama in background and save its PID
    ollama run "$model" &
    OLLAMA_PID=$!
    
    # Wait a moment for Ollama to initialize
    sleep 5
    
    echo "Running benchmark for Ollama model: $model"
    cargo run --bin goose -- bench --suites small_models
    
    # Kill the Ollama process
    echo "Stopping Ollama process for: $model"
    # ollama stop "$model"
    kill $OLLAMA_PID
    
    # Wait a moment to ensure process is cleaned up
    sleep 2
}

# Function to run benchmark for a Databricks model
run_databricks_benchmark() {
    local model=$1
    echo "Running benchmark for Databricks model: $model"
    
    # Set the environment variables
    export GOOSE_PROVIDER="databricks"
    export GOOSE_MODEL="$model"
    
    cargo run --bin goose -- bench --suites small_models
}

# Run Ollama benchmarks
# ollama_models=("llama3.3" "mistral" "qwen2.5" "qwen2.5-coder")
ollama_models=("qwen2.5")
echo "=== Running Ollama Models ==="
for model in "${ollama_models[@]}"; do
    echo "==============================================="
    echo "Starting benchmark suite for Ollama model: $model"
    echo "==============================================="
    run_ollama_benchmark "$model"
    echo "Completed benchmark suite for Ollama model: $model"
    echo "==============================================="
done

# # Run Databricks benchmarks
# databricks_models=("claude-3-5-haiku" "claude-3-5-sonnet-2" "gpt-4o" "gpt-4o-mini")
# echo "=== Running Databricks Models ==="
# for model in "${databricks_models[@]}"; do
#     echo "==============================================="
#     echo "Starting benchmark suite for Databricks model: $model"
#     echo "==============================================="
#     run_databricks_benchmark "$model"
#     echo "Completed benchmark suite for Databricks model: $model"
#     echo "==============================================="
# done

echo "All benchmark runs completed!"