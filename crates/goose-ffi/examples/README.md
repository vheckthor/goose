# Goose FFI Examples

This directory contains examples of using the Goose FFI library from Python. These examples demonstrate how to integrate Goose with applications in other languages through Foreign Function Interface (FFI).

## Non-Yielding API Example

The non-yielding API is designed for scenarios where yielding/streaming behavior is not possible or desired, such as when crossing language boundaries through FFI. This is particularly useful for Kotlin, C++, or other language integrations where asynchronous yields are difficult to implement.

### Key Files

- `test_non_yielding.py`: Demonstrates the non-yielding API with examples of simple conversation and tool usage
- `goose_agent.py`: Contains the main GooseAgent class with implementations for all API types
- `handle_ffi.py`: Utility functions for safely managing FFI memory and data conversion
- `build_and_test.sh`: Script to build the library and run the tests

### Setting Up

1. Create a `.env` file with your API credentials:

```
DATABRICKS_API_KEY=your_api_key
DATABRICKS_HOST=your_databricks_host
```

2. Install dependencies:

```bash
pip install python-dotenv
```

3. Build the library and run the tests:

```bash
./build_and_test.sh
```

### API Modes

The example supports three modes of interaction:

1. **Streaming Mode**: Uses a streaming API without tool support
2. **Non-streaming with Tools (Yielding)**: Step-by-step processing with tool calls using yields
3. **Non-yielding with Tools**: Single-call processing without yielding for FFI boundaries

### Using in Kotlin/JNI

To use this API from Kotlin:

1. Load the native library using JNA or JNI
2. Define the function signatures to match the C functions
3. Use the non-yielding API to make calls without needing to yield

Example Kotlin function (pseudo-code):

```kotlin
fun sendMessageToAgent(agentPtr: Long, message: String, toolResponses: List<Pair<String, String>>): String {
    // Convert message to JSON
    val messagesJson = "[{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"$message\"}]}]"
    val toolRequestsJson = "[]"
    val toolResponsesJson = if (toolResponses.isEmpty()) "[]" else toolResponsesToJson(toolResponses)
    
    // Call the non-yielding API
    val responsePtr = gooseAgentReplyNonYielding(
        agentPtr,
        messagesJson,
        toolRequestsJson,
        toolResponsesJson
    )
    
    // Convert response back to string
    val response = convertResponseToString(responsePtr)
    gooseFreeString(responsePtr)
    
    return response
}
```