# DeepSeek Model Integration Notes

## Overview
This document summarizes findings and solutions for integrating DeepSeek models via the HuggingFace provider in Goose.

## Issues Identified

1. **Tool Choice Parameter Required**
   - DeepSeek models require a `tool_choice` parameter when tools are provided in the request.
   - Unlike other models that support `"auto"` as a value, DeepSeek models require either:
     - `"none"` to disable automatic tool calling
     - A specific tool definition object to force using that tool

2. **Conversation Loop Limitations**
   - DeepSeek models have difficulty handling the full conversation loop with tool calling.
   - After receiving a tool response, the model encounters errors when attempting to continue the conversation.
   - Error: `"Invalid request. Please check the parameters and try again. Details: can only concatenate str (not \"dict\") to str"`

## Solutions Implemented

1. **Tool Choice Parameter**
   - Added logic to detect when tools are present and add the appropriate `tool_choice` parameter.
   - For the initial query, we use a specific tool choice for `developer__shell` to encourage the model to use it.
   - For subsequent requests after tool responses, we switch to `"none"` to prevent errors.

2. **Tool Response Handling**
   - Modified the `complete` method to detect when we're handling a tool response.
   - For tool responses, we create a simplified payload without tools to avoid the concatenation error.
   - This allows the model to respond to the tool output without trying to call more tools.

## Current Status

- Basic tool calling works - the model can successfully use the `developer__shell` tool.
- The conversation still ends after the first tool response due to API limitations.
- This is a known limitation of the DeepSeek models through the HuggingFace API.

## Future Improvements

1. Make the tool_choice configurable via an environment variable.
2. Add better error handling for specific DeepSeek error messages.
3. Implement a fallback mechanism to disable tools when the model doesn't support them.
4. Explore if newer DeepSeek model versions improve tool calling support.

## References

- HuggingFace API Documentation
- DeepSeek Model Documentation
- Error message: `"Invalid request. Please check the parameters and try again. Details: can only concatenate str (not \"dict\") to str"`