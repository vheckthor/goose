#!/bin/bash
set -e

# Change to the repository root
cd "$(dirname "$0")/../../../"

echo "Building Goose FFI library for local machine..."
cargo build -p goose-ffi --release

# Verify the library was built
LIB_NAME=""
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_NAME="libgoose_ffi.dylib"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    LIB_NAME="libgoose_ffi.so"
else
    echo "Unsupported platform: $OSTYPE"
    exit 1
fi

if [ ! -f "target/release/$LIB_NAME" ]; then
    echo "Error: Library build failed, $LIB_NAME not found!"
    exit 1
fi

# Copy the library to the examples directory for easy testing
cp "target/release/$LIB_NAME" "crates/goose-ffi/examples/"

echo "Library built successfully and copied to examples directory."

# Move to the examples directory
cd crates/goose-ffi/examples

# Make the test script executable
chmod +x test_non_yielding.py

# Check if .env file exists
if [ -f ".env" ]; then
    echo "Found .env file, running test script..."
    python3 test_non_yielding.py
elif [[ -n "$DATABRICKS_API_KEY" && -n "$DATABRICKS_HOST" ]]; then
    echo "Using environment variables from shell, running test script..."
    python3 test_non_yielding.py
else
    echo "No .env file found and no environment variables set."
    echo "Please create a .env file with DATABRICKS_API_KEY and DATABRICKS_HOST or set these environment variables."
    echo "Then run: python3 test_non_yielding.py"
fi