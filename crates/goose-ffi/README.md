# Goose FFI

Foreign Function Interface (FFI) for the Goose AI agent framework, allowing integration with other programming languages.

## Overview

The Goose FFI library provides C-compatible bindings for the Goose AI agent framework, enabling you to:

- Create and manage Goose agents from any language with C FFI support
- Configure and use the Databricks AI provider for now but is extensible to other providers as needed
- Send messages to agents and receive responses
- Register custom tools that can be invoked by the AI agent

## Building

To build the FFI library, you'll need Rust and Cargo installed. Then run:

```bash
# Build the library in debug mode
cargo build

# Build the library in release mode (recommended for production)
cargo build --release
```

This will generate a dynamic library (.so, .dll, or .dylib depending on your platform) in the `target` directory, and automatically generate the C header file in the `include` directory.

## Generated C Header

The library uses cbindgen to automatically generate a C header file (`goose_ffi.h`) during the build process. This header contains all the necessary types and function declarations to use the library from C or any language with C FFI support.

## Examples

The FFI library includes examples in multiple languages to demonstrate how to use it.

### Python Example

The `examples/goose_agent.py` demonstrates using the FFI library from Python with ctypes. It shows:

1. How to create a proper Python wrapper around the Goose FFI interface
2. Loading the shared library dynamically based on platform
3. Setting up C-compatible structures
4. Creating an object-oriented API for easier use

The example demonstrates how to use tool callbacks to enable the agent to invoke functions in your application.

To run the Python example:

```bash
# First, build the FFI library
cd /path/to/goose/crates/goose-ffi
cargo build

# Then run the Python example
cd examples
python goose_agent.py
```

You need to have Python 3.6+ installed with the `ctypes` module (included in standard library).


### Tool Agent Examples

The library includes examples demonstrating how to implement and register tools that can be invoked by the AI agent:

- `examples/tool_agent.c` - C example showing how to implement a calculator tool
- `examples/tool_agent.py` - Python equivalent with the same calculator functionality

To build and run the C tool agent example:

```bash
cd examples
make run-tool-agent
```

To run the Python tool agent example:

```bash
cd examples
make run-python-tool-agent
```

Example interaction:

```
> Calculate 5 + 3
Agent: The sum of 5 and 3 is 8.

> What is 10 divided by 2?
Agent: 10 divided by 2 equals 5.
```

The agent will use the calculator tool to perform the requested operations.

## Using from Other Languages

The Goose FFI library can be used from many programming languages with C FFI support, including:

- Python (via ctypes or cffi)
- JavaScript/Node.js (via node-ffi)
- Ruby (via fiddle)
- C#/.NET (via P/Invoke)
- Go (via cgo)
- Java / Kotlin (via JNA or JNI)

Check the documentation for FFI support in your language of choice for details on how to load and use a C shared library.

## Provider Configuration

The FFI interface uses a provider type enumeration to specify which AI provider to use:

```c
// C enum (defined in examples/simple_agent.c)
typedef enum {
    PROVIDER_DATABRICKS = 0,  // Databricks AI provider
} ProviderType;
```

```python
# Python enum (defined in examples/goose_agent.py)
class ProviderType(IntEnum):
    DATABRICKS = 0  # Databricks AI provider
```

Currently, only the Databricks provider (provider_type = 0) is supported. If you attempt to use any other provider type, an error will be returned.

### Environment-based Configuration

The library supports configuration via environment variables, which makes it easier to use in containerized or CI/CD environments without hardcoding credentials:

#### Databricks Provider (type = 0)

```
DATABRICKS_API_KEY=dapi...     # Databricks API key
DATABRICKS_HOST=...            # Databricks host URL (e.g., "https://your-workspace.cloud.databricks.com")
```

These environment variables will be used automatically if you don't provide the corresponding parameters when creating an agent.

## Tool Callbacks

The FFI library supports registering custom tools that can be invoked by the AI agent. This enables the agent to perform actions in your application based on user requests.

### Creating a Tool

To create a tool:

1. Define the tool's parameters using the `ToolParamDef` structure
2. Create a tool schema using `goose_create_tool_schema`
3. Register the tool with the agent using `goose_agent_register_tool_callback`
4. Implement a callback function to handle tool invocations

The callback function has the following signature:

```c
char* tool_callback(size_t param_count, const goose_ToolParam* params, void* user_data);
```

- `param_count`: Number of parameters passed to the tool
- `params`: Array of parameter name-value pairs
- `user_data`: User-provided data pointer passed during registration
- Return value: JSON-formatted string with the tool's result (must be allocated with malloc or equivalent)

### Tool Parameter Types

Tools can have parameters of the following types:

- String (ToolParamType.String = 0)
- Number (ToolParamType.Number = 1)
- Boolean (ToolParamType.Boolean = 2)
- Array (ToolParamType.Array = 3)
- Object (ToolParamType.Object = 4)

Parameters can be marked as required or optional.

## Thread Safety

The FFI library is designed to be thread-safe. Each agent instance is independent, and tool callbacks are handled in a thread-safe manner. However, the same agent instance should not be used from multiple threads simultaneously without external synchronization.

## Error Handling

Functions that can fail return either null pointers or special result structures that indicate success or failure. Always check return values and clean up resources using the appropriate free functions.

## Memory Management

The FFI interface handles memory allocation and deallocation. Use the provided free functions (like `goose_free_string` and `goose_free_async_result`) to release resources when you're done with them.
