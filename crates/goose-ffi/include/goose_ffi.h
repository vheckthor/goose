#ifndef GOOSE_FFI_H
#define GOOSE_FFI_H

/* Goose FFI - C interface for the Goose AI agent framework */


#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/*
 Stream state for managing ongoing conversation
 */
typedef struct goose_StreamState goose_StreamState;

/*
 Result type for async operations

 - succeeded: true if the operation succeeded, false otherwise
 - error_message: Error message if succeeded is false, NULL otherwise
 */
typedef struct goose_AsyncResult {
  bool succeeded;
  char *error_message;
} goose_AsyncResult;

/*
 Opaque pointer to Agent
 */
typedef struct goose_AgentPtr {
  goose_Agent *_0;
} goose_AgentPtr;

/*
 Provider configuration used to initialize an AI provider

 - provider_type: Provider type (0 = Databricks, other values will produce an error)
 - api_key: Provider API key (null for default from environment variables)
 - model_name: Model name to use (null for provider default)
 - host: Provider host URL (null for default from environment variables)
 */
typedef struct goose_ProviderConfigFFI {
  uint32_t provider_type;
  const char *api_key;
  const char *model_name;
  const char *host;
} goose_ProviderConfigFFI;

/*
 Extension configuration used to initialize an extension for an agent

 - name: Extension name
 - config_json: JSON configuration for the extension (null for default)
 */
typedef struct goose_ExtensionConfigFFI {
  const char *name;
  const char *config_json;
} goose_ExtensionConfigFFI;

/*
 Opaque pointer to StreamState
 */
typedef struct goose_StreamStatePtr {
  struct goose_StreamState *_0;
} goose_StreamStatePtr;

/*
 Message structure for agent interactions

 - role: 0 = user, 1 = assistant, 2 = system
 - content: Text content of the message
 */
typedef struct goose_MessageFFI {
  uint32_t role;
  const char *content;
} goose_MessageFFI;

/*
 Free an async result structure

 This function frees the memory allocated for an AsyncResult structure,
 including any error message it contains.

 # Safety

 The result pointer must be a valid pointer returned by a goose FFI function,
 or NULL.
 */
void goose_free_async_result(struct goose_AsyncResult *result);

/*
 Create a new agent with the given provider configuration

 # Parameters

 - config: Provider configuration
 - extension_config: Extension configuration (can be NULL if no extension is needed)

 # Returns

 A new agent pointer, or a null pointer if creation failed

 # Safety

 The config pointer must be valid or NULL. The resulting agent must be freed
 with goose_agent_free when no longer needed.
 */
struct goose_AgentPtr goose_agent_new(const struct goose_ProviderConfigFFI *config,
                                      const struct goose_ExtensionConfigFFI *extension_config);

/*
 Free an agent

 This function frees the memory allocated for an agent.

 # Parameters

 - agent_ptr: Agent pointer returned by goose_agent_new

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new,
 or have a null internal pointer. The agent_ptr must not be used after
 calling this function.
 */
void goose_agent_free(struct goose_AgentPtr agent_ptr);

/*
 Send a message to the agent and get the response

 This function sends a message to the agent and returns the response.

 # Parameters

 - agent_ptr: Agent pointer
 - message: Message to send

 # Returns

 A C string with the agent's response, or NULL on error.
 This string must be freed with goose_free_string when no longer needed.

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The message must be a valid C string.
 */
char *goose_agent_send_message(struct goose_AgentPtr agent_ptr, const char *message);

/*
 Free a string allocated by goose FFI functions

 This function frees memory allocated for strings returned by goose FFI functions.

 # Parameters

 - s: String to free

 # Safety

 The string must have been allocated by a goose FFI function, or be NULL.
 The string must not be used after calling this function.
 */
void goose_free_string(char *s);

/*
 Create a new stream state for an agent

 This function creates a new stream state for an agent, which can be used
 to manage an ongoing conversation with streaming responses.

 Note: This function only creates the stream state container but does not
 initialize an active stream. You must call goose_stream_send_message
 before calling goose_stream_next to create an active stream.

 # Parameters

 - agent_ptr: Agent pointer

 # Returns

 A new stream state pointer, or NULL on error.

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The resulting stream state must be freed with goose_stream_free when no longer needed.
 */
struct goose_StreamStatePtr goose_stream_new(struct goose_AgentPtr agent_ptr);

/*
 Free a stream state

 This function frees the memory allocated for a stream state.

 # Parameters

 - stream_ptr: Stream state pointer

 # Safety

 The stream_ptr must be a valid pointer returned by goose_stream_new,
 or have a null internal pointer. The stream_ptr must not be used after
 calling this function.
 */
void goose_stream_free(struct goose_StreamStatePtr stream_ptr);

/*
 Get the next message from the stream

 This function gets the next message from the stream. If there are no more
 messages, it returns NULL.

 # Parameters

 - stream_ptr: Stream state pointer

 # Returns

 A pointer to a MessageFFI struct, or NULL if there are no more messages, no active stream, or an error occurred.
 The message must be freed with goose_free_message when no longer needed.

 # Safety

 The stream_ptr must be a valid pointer returned by goose_stream_new.
 */
struct goose_MessageFFI *goose_stream_next(struct goose_StreamStatePtr stream_ptr);

/*
 Submit a tool result to the stream

 This function submits a tool result to the stream, which will be used by the agent
 to continue the conversation.

 # Parameters

 - stream_ptr: Stream state pointer
 - tool_id: Tool ID
 - result_json: Tool result as JSON

 # Returns

 An AsyncResult struct with the result of the operation.

 # Safety

 The stream_ptr must be a valid pointer returned by goose_stream_new.
 The tool_id and result_json must be valid C strings.
 */
struct goose_AsyncResult *goose_stream_submit_tool_result(struct goose_StreamStatePtr stream_ptr,
                                                          const char *tool_id,
                                                          const char *result_json);

/*
 Free a message

 This function frees the memory allocated for a message.

 # Parameters

 - message: Message pointer

 # Safety

 The message must be a valid pointer returned by goose_stream_next,
 or NULL. The message must not be used after calling this function.
 */
void goose_free_message(struct goose_MessageFFI *message);

/*
 Send a message to an ongoing stream

 This function sends a message to an ongoing stream,
 which will be used by the agent to continue the conversation.
 If no stream exists yet, it will create a new one.

 # Parameters

 - stream_ptr: Stream state pointer
 - message: Message to send

 # Returns

 An AsyncResult struct with the result of the operation.

 # Safety

 The stream_ptr must be a valid pointer returned by goose_stream_new.
 The message must be a valid C string.
 */
struct goose_AsyncResult *goose_stream_send_message(struct goose_StreamStatePtr stream_ptr,
                                                    const char *message);

#endif // GOOSE_FFI_H
