use futures::future;
use std::collections::HashSet;
use tracing::{debug, instrument};

use crate::agents::extension_manager::ExtensionManager;
use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::message::{Message, ToolRequest};
use crate::permission::permission_judge::check_tool_permissions;
use crate::permission::Permission;
use mcp_core::{tool::ToolCall, Content, ToolError};

use crate::agents::Agent;

impl Agent {
    /// Handle frontend tool requests
    /// Returns a Message with tool responses for the frontend tools
    pub async fn handle_frontend_requests(&self, frontend_requests: &[ToolRequest]) -> Message {
        let mut message_tool_response = Message::user();

        for request in frontend_requests {
            if let Ok(tool_call) = request.tool_call.clone() {
                // Send frontend tool request and wait for response
                yield_message_with_frontend_tool_request(request.id.clone(), tool_call.clone());

                if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                    message_tool_response = message_tool_response.with_tool_response(id, result);
                }
            }
        }

        message_tool_response
    }

    /// Handle enable extension requests
    /// Returns a Message with tool responses and a boolean indicating if any extensions were enabled
    pub async fn handle_enable_extension_requests(
        &self,
        enable_extension_requests: &[ToolRequest],
        extension_manager: &mut ExtensionManager,
    ) -> (Message, bool) {
        let mut message_tool_response = Message::user();
        let mut install_results = Vec::new();
        let mut any_enabled = false;

        for request in enable_extension_requests {
            if let Ok(tool_call) = request.tool_call.clone() {
                let confirmation = Message::user().with_enable_extension_request(
                    request.id.clone(),
                    tool_call
                        .arguments
                        .get("extension_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                );
                yield_message(confirmation);

                let mut rx = self.confirmation_rx.lock().await;
                while let Some((req_id, extension_confirmation)) = rx.recv().await {
                    if req_id == request.id {
                        if extension_confirmation.permission == Permission::AllowOnce
                            || extension_confirmation.permission == Permission::AlwaysAllow
                        {
                            let extension_name = tool_call
                                .arguments
                                .get("extension_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let install_result = Self::enable_extension(
                                extension_manager,
                                extension_name,
                                request.id.clone(),
                            )
                            .await;
                            install_results.push(install_result);
                            any_enabled = true;
                        }
                        break;
                    }
                }
            }
        }

        // Process install results
        for (request_id, output) in install_results {
            message_tool_response = message_tool_response.with_tool_response(request_id, output);
        }

        (message_tool_response, any_enabled)
    }

    /// Handle regular tool requests based on permission mode
    /// Returns a Message with tool responses
    pub async fn handle_regular_tool_requests(
        &self,
        tool_requests: &[ToolRequest],
        mode: &str,
        tools_with_readonly_annotation: HashSet<String>,
        tools_without_annotation: HashSet<String>,
        extension_manager: &ExtensionManager,
    ) -> Message {
        let mut message_tool_response = Message::user();

        // If mode is chat, return a message indicating tools were skipped
        if mode == "chat" {
            for request in tool_requests {
                // Skip search extension requests since they were already processed
                if let Ok(tool_call) = &request.tool_call {
                    if tool_call.name == super::super::platform_tools::PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
                        continue;
                    }
                }

                message_tool_response = message_tool_response.with_tool_response(
                    request.id.clone(),
                    Ok(vec![Content::text(
                        "Let the user know the tool call was skipped in Goose chat mode. \
                        DO NOT apologize for skipping the tool call. DO NOT say sorry. \
                        Provide an explanation of what the tool call would do, structured as a \
                        plan for the user. Again, DO NOT apologize. \
                        **Example Plan:**\n \
                        1. **Identify Task Scope** - Determine the purpose and expected outcome.\n \
                        2. **Outline Steps** - Break down the steps.\n \
                        If needed, adjust the explanation based on user preferences or questions.",
                    )]),
                );
            }
            return message_tool_response;
        }

        // For auto mode or approval modes
        if mode == "auto" || mode == "approve" || mode == "smart_approve" {
            let mut permission_manager = PermissionManager::default();

            // Skip the platform tools for permission checks
            let filtered_requests: Vec<&ToolRequest> = tool_requests
                .iter()
                .filter(|req| {
                    if let Ok(tool_call) = &req.tool_call {
                        !tool_call.name.starts_with("platform__")
                    } else {
                        true // If there's an error (Err), don't skip the request
                    }
                })
                .collect();

            let permission_check_result = check_tool_permissions(
                filtered_requests,
                mode,
                tools_with_readonly_annotation.clone(),
                tools_without_annotation.clone(),
                &mut permission_manager,
                self.provider(),
            )
            .await;

            // Handle pre-approved and read-only tools in parallel
            let mut tool_futures = Vec::new();

            // Process approved tools
            for request in &permission_check_result.approved {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let is_frontend_tool = self.is_frontend_tool(&tool_call.name);
                    let tool_future = Self::create_tool_future(
                        extension_manager,
                        tool_call,
                        is_frontend_tool,
                        request.id.clone(),
                    );
                    tool_futures.push(tool_future);
                }
            }

            // Process denied tools
            let denied_content_text = Content::text(
                "The user has declined to run this tool. \
                DO NOT attempt to call this tool again. \
                If there are no alternative methods to proceed, clearly explain the situation and STOP."
            );

            for request in &permission_check_result.denied {
                message_tool_response = message_tool_response
                    .with_tool_response(request.id.clone(), Ok(vec![denied_content_text.clone()]));
            }

            // Process tools that need approval
            for request in &permission_check_result.needs_approval {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let is_frontend_tool = self.is_frontend_tool(&tool_call.name);
                    let confirmation = Message::user().with_tool_confirmation_request(
                        request.id.clone(),
                        tool_call.name.clone(),
                        tool_call.arguments.clone(),
                        Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
                    );
                    yield_message(confirmation);

                    // Wait for confirmation response through the channel
                    let mut rx = self.confirmation_rx.lock().await;
                    while let Some((req_id, tool_confirmation)) = rx.recv().await {
                        if req_id == request.id {
                            let confirmed = tool_confirmation.permission == Permission::AllowOnce
                                || tool_confirmation.permission == Permission::AlwaysAllow;
                            if confirmed {
                                // Add this tool call to the futures collection
                                let tool_future = Self::create_tool_future(
                                    extension_manager,
                                    tool_call.clone(),
                                    is_frontend_tool,
                                    request.id.clone(),
                                );
                                tool_futures.push(tool_future);
                                if tool_confirmation.permission == Permission::AlwaysAllow {
                                    permission_manager.update_user_permission(
                                        &tool_call.name,
                                        PermissionLevel::AlwaysAllow,
                                    );
                                }
                            } else {
                                // User declined - add declined response
                                message_tool_response = message_tool_response.with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![denied_content_text.clone()]),
                                );
                            }
                            break; // Exit the loop once the matching `req_id` is found
                        }
                    }
                }
            }

            // Wait for all tool calls to complete
            let results = future::join_all(tool_futures).await;
            for (request_id, output) in results {
                message_tool_response =
                    message_tool_response.with_tool_response(request_id, output);
            }
        }

        message_tool_response
    }

    /// Handle search extension requests
    /// Returns a Message with tool responses
    pub async fn handle_search_extension_requests(
        &self,
        search_requests: &[ToolRequest],
        extension_manager: &ExtensionManager,
    ) -> Message {
        let mut message_tool_response = Message::user();
        let mut tool_futures = Vec::new();

        for request in search_requests {
            if let Ok(tool_call) = request.tool_call.clone() {
                let is_frontend_tool = self.is_frontend_tool(&tool_call.name);
                let tool_future = Self::create_tool_future(
                    extension_manager,
                    tool_call,
                    is_frontend_tool,
                    request.id.clone(),
                );
                tool_futures.push(tool_future);
            }
        }

        // Wait for all tool calls to complete
        let results = future::join_all(tool_futures).await;
        for (request_id, output) in results {
            message_tool_response = message_tool_response.with_tool_response(request_id, output);
        }

        message_tool_response
    }

    /// Create a future that will execute a tool call
    #[instrument(skip(tool_call, extension_manager, request_id), fields(input, output))]
    pub async fn create_tool_future(
        extension_manager: &ExtensionManager,
        tool_call: ToolCall,
        is_frontend_tool: bool,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let result = if tool_call.name
            == super::super::platform_tools::PLATFORM_READ_RESOURCE_TOOL_NAME
        {
            // Check if the tool is read_resource and handle it separately
            extension_manager
                .read_resource(tool_call.arguments.clone())
                .await
        } else if tool_call.name == super::super::platform_tools::PLATFORM_LIST_RESOURCES_TOOL_NAME
        {
            extension_manager
                .list_resources(tool_call.arguments.clone())
                .await
        } else if tool_call.name
            == super::super::platform_tools::PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME
        {
            extension_manager.search_available_extensions().await
        } else if is_frontend_tool {
            // For frontend tools, return an error indicating we need frontend execution
            Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            ))
        } else {
            extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        (request_id, result)
    }

    /// Enable an extension
    pub async fn enable_extension(
        extension_manager: &mut ExtensionManager,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let config =
            match crate::config::ExtensionConfigManager::get_config_by_name(&extension_name) {
                Ok(Some(config)) => config,
                Ok(None) => {
                    return (
                        request_id,
                        Err(ToolError::ExecutionError(format!(
                        "Extension '{}' not found. Please check the extension name and try again.",
                        extension_name
                    ))),
                    )
                }
                Err(e) => {
                    return (
                        request_id,
                        Err(ToolError::ExecutionError(format!(
                            "Failed to get extension config: {}",
                            e
                        ))),
                    )
                }
            };

        let result = extension_manager
            .add_extension(config)
            .await
            .map(|_| {
                vec![Content::text(format!(
                    "The extension '{}' has been installed successfully",
                    extension_name
                ))]
            })
            .map_err(|e| ToolError::ExecutionError(e.to_string()));

        (request_id, result)
    }
}

// Helper function for yielding messages in async_stream context
// This is a placeholder that will be replaced in the main reply method
fn yield_message(_message: Message) {
    // This is just a placeholder - the actual yield happens in the async_stream macro
}

// Helper function for yielding frontend tool requests
// This is a placeholder that will be replaced in the main reply method
fn yield_message_with_frontend_tool_request(_id: String, _tool_call: ToolCall) {
    // This is just a placeholder - the actual yield happens in the async_stream macro
}
