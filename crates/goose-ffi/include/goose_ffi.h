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
 Provider Type enumeration
 Currently only Databricks is supported
 */
enum goose_ProviderType {
  /*
   Databricks AI provider
   */
  goose_ProviderType_Databricks = 0,
};
typedef uint32_t goose_ProviderType;

/*
 Result status for reply step operations
 */
enum goose_ReplyStatus {
  /*
   Reply is complete, no more steps needed
   */
  goose_ReplyStatus_Complete = 0,
  /*
   Tool call needed, waiting for tool result
   */
  goose_ReplyStatus_ToolCallNeeded = 1,
  /*
   Error occurred
   */
  goose_ReplyStatus_Error = 2,
};
typedef uint32_t goose_ReplyStatus;

/*
 Represents the state of the agent's reply process
 */
typedef struct goose_AgentReplyState goose_AgentReplyState;

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
 Pointer type for the agent
 */
typedef goose_Agent *goose_AgentPtr;

/*
 Provider configuration used to initialize an AI provider

 - provider_type: Provider type (0 = Databricks, other values will produce an error)
 - api_key: Provider API key (null for default from environment variables)
 - model_name: Model name to use (null for provider default)
 - host: Provider host URL (null for default from environment variables)
 - ephemeral: Whether to use ephemeral in-memory configuration (true) or persistent configuration (false)
 */
typedef struct goose_ProviderConfigFFI {
  goose_ProviderType provider_type;
  const char *api_key;
  const char *model_name;
  const char *host;
  bool ephemeral;
} goose_ProviderConfigFFI;

typedef struct goose_AgentReplyState *goose_AgentReplyStatePtr;

/*
 Tool call information
 */
typedef struct goose_ToolCallFFI {
  char *id;
  char *tool_name;
  char *arguments_json;
} goose_ToolCallFFI;

/*
 Reply step result
 */
typedef struct goose_ReplyStepResult {
  goose_ReplyStatus status;
  char *message;
  struct goose_ToolCallFFI tool_call;
} goose_ReplyStepResult;

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

 # Returns

 A new agent pointer, or a null pointer if creation failed

 # Safety

 The config pointer must be valid or NULL. The resulting agent must be freed
 with goose_agent_free when no longer needed.
 */
goose_AgentPtr goose_agent_new(const struct goose_ProviderConfigFFI *config);

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
void goose_agent_free(goose_AgentPtr agent_ptr);

/*
 Send a message to the agent and get the response

 This function sends a message to the agent and returns the response.
 Tool handling is not yet supported and will be implemented in a future commit
 so this may change significantly

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
char *goose_agent_send_message(goose_AgentPtr agent_ptr, const char *message);

/*
 Begin a new non-streaming reply conversation with the agent

 This function starts a new conversation and returns a state pointer that can be used
 to continue the conversation step-by-step with goose_agent_reply_step

 # Parameters

 - agent_ptr: Agent pointer
 - message: Message to send

 # Returns

 A new agent reply state pointer, or NULL on error.
 This pointer must be freed with goose_agent_reply_state_free when no longer needed.

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The message must be a valid C string.
 */
goose_AgentReplyStatePtr goose_agent_reply_begin(goose_AgentPtr agent_ptr, const char *message);

/*
 Execute one step of the reply process

 This function processes one step of the reply process. If the status is Complete,
 the reply is done. If the status is ToolCallNeeded, the tool call information is
 filled in and the caller should execute the tool and provide the result with
 goose_agent_reply_tool_result.

 # Parameters

 - state_ptr: Agent reply state pointer

 # Returns

 A ReplyStepResult struct with the status, message, and tool call information.
 The message and tool call fields must be freed with goose_free_string when
 no longer needed.

 # Safety

 The state_ptr must be a valid pointer returned by goose_agent_reply_begin
 or goose_agent_reply_tool_result.
 */
struct goose_ReplyStepResult goose_agent_reply_step(goose_AgentReplyStatePtr state_ptr);

/*
 Provide a tool result to continue the reply process

 This function provides a tool result to the agent and continues the reply process.
 It returns a new state pointer that can be used to continue the conversation.

 # Parameters

 - state_ptr: Agent reply state pointer
 - tool_id: Tool ID from the previous step
 - result: Tool result

 # Returns

 A new agent reply state pointer, or NULL on error.
 This pointer must be freed with goose_agent_reply_state_free when no longer needed.

 # Safety

 The state_ptr must be a valid pointer returned by goose_agent_reply_begin
 or goose_agent_reply_tool_result.
 The tool_id and result must be valid C strings.
 */
goose_AgentReplyStatePtr goose_agent_reply_tool_result(goose_AgentReplyStatePtr state_ptr,
                                                       const char *tool_id,
                                                       const char *result);

/*
 Free an agent reply state

 This function frees the memory allocated for an agent reply state.

 # Parameters

 - state_ptr: Agent reply state pointer

 # Safety

 The state_ptr must be a valid pointer returned by goose_agent_reply_begin
 or goose_agent_reply_tool_result.
 The state_ptr must not be used after calling this function.
 */
void goose_agent_reply_state_free(goose_AgentReplyStatePtr state_ptr);

/*
 Free a tool call

 This function frees the memory allocated for a tool call.

 # Parameters

 - tool_call: Tool call to free

 # Safety

 The tool_call must have been allocated by a goose FFI function.
 The tool_call must not be used after calling this function.
 */
void goose_free_tool_call(struct goose_ToolCallFFI tool_call);

/*
 Register tools with the agent

 This function registers tools with the agent for use with the non-streaming API.
 The tools should be provided as a JSON array of Tool objects.

 # Parameters

 - agent_ptr: Agent pointer
 - tools_json: JSON string containing an array of Tool objects
 - extension_name: Optional name for the extension. If NULL, a default name will be used.
 - instructions: Optional instructions for using the tools. If NULL, default instructions will be used.

 # Returns

 A boolean indicating success (true) or failure (false)

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The tools_json must be a valid JSON string in the expected format.
 The extension_name and instructions must be valid UTF-8 strings or NULL.
 */
bool goose_agent_register_tools(goose_AgentPtr agent_ptr,
                                const char *tools_json,
                                const char *extension_name,
                                const char *instructions);

/*
 Execute a non-yielding reply with tool requests and responses

 This function executes a complete conversation with the agent, including tool calls,
 and returns the final response. Unlike the stream-based reply function, this method
 requires the caller to provide both tool requests and their responses upfront.

 # Parameters

 - agent_ptr: Agent pointer
 - messages_json: JSON string containing an array of message objects
 - tool_requests_json: JSON string containing an array of tool request objects (can be empty array)
 - tool_responses_json: JSON string containing an array of tool response objects (can be empty array)

 # Returns

 A C string containing the agent's response (must be freed with goose_free_string)
 or NULL on error

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The messages_json, tool_requests_json, and tool_responses_json must be valid JSON strings.
 */
char *goose_agent_reply_non_yielding(goose_AgentPtr agent_ptr,
                                     const char *messages_json,
                                     const char *tool_requests_json,
                                     const char *tool_responses_json);

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

#endif // GOOSE_FFI_H
