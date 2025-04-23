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
echo "To run the example, set your credentials and execute:"
echo "cd crates/goose-ffi/examples && DATABRICKS_API_KEY=your_key DATABRICKS_HOST=your_host python3 goose_agent.py"

if [[ -n "$DATABRICKS_API_KEY" && -n "$DATABRICKS_HOST" ]]; then
    echo "Detected environment variables, running example now..."
    cd crates/goose-ffi/examples
    python3 goose_agent.py
fi