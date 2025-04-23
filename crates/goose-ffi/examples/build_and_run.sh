#!/bin/bash
set -e

# Change to the repository root
cd "$(dirname "$0")/../../../"

echo "Building Goose FFI library for Linux x86_64..."
CROSS_BUILD_OPTS="--platform linux/amd64 --no-cache" CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build -p goose-ffi --release --target x86_64-unknown-linux-gnu

# Verify the library was built
if [ ! -f "target/x86_64-unknown-linux-gnu/release/libgoose_ffi.so" ]; then
    echo "Error: Library build failed, libgoose_ffi.so not found!"
    exit 1
fi

echo "Building Docker image..."
docker build -t goose-agent --platform=linux/amd64 -f crates/goose-ffi/examples/Dockerfile .

# Check if DATABRICKS_API_KEY and DATABRICKS_HOST environment variables are set
if [ -z "$DATABRICKS_API_KEY" ] || [ -z "$DATABRICKS_HOST" ]; then
    echo "Error: DATABRICKS_API_KEY and DATABRICKS_HOST environment variables must be set."
    echo "Example usage:"
    echo "DATABRICKS_API_KEY=your_api_key DATABRICKS_HOST=your_host ./crates/goose-ffi/examples/build_and_run.sh"
    exit 1
fi

echo "Running Docker container..."
docker run -it --read-only --platform=linux/amd64 \
    -e DATABRICKS_API_KEY="${DATABRICKS_API_KEY}" \
    -e DATABRICKS_HOST="${DATABRICKS_HOST}" \
    -e RUST_BACKTRACE=full \
    goose-agent

echo "Container execution completed."