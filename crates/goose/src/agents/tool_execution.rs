use mcp_core::ToolCall;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::debug;

use async_stream::try_stream;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use tokio::sync::Mutex;

use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::message::{Message, ToolRequest};
use crate::permission::Permission;
use mcp_core::{Content, ToolError};

// Type alias for ToolFutures - used in the agent loop to join all futures together
pub(crate) type ToolFuture<'a> =
    Pin<Box<dyn Future<Output = (String, Result<Vec<Content>, ToolError>)> + Send + 'a>>;
pub(crate) type ToolFuturesVec<'a> = Arc<Mutex<Vec<ToolFuture<'a>>>>;
// Type alias for extension installation results
pub(crate) type ExtensionInstallResult = (String, Result<Vec<Content>, ToolError>);
pub(crate) type ExtensionInstallResults = Arc<Mutex<Vec<ExtensionInstallResult>>>;

use crate::agents::Agent;

use super::ExtensionManager;

impl Agent {
    pub(crate) fn handle_approval_tool_requests<'a>(
        &'a self,
        //tool_requests: &'a [ToolRequest],
        tool_requests: Vec<ToolRequest>,
        tool_futures: ToolFuturesVec<'a>,
        permission_manager: &'a mut PermissionManager,
        message_tool_response: Arc<Mutex<Message>>,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            for request in tool_requests {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let confirmation = Message::user().with_tool_confirmation_request(
                        request.id.clone(),
                        tool_call.name.clone(),
                        tool_call.arguments.clone(),
                        Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
                    );

                    yield confirmation;

                    let mut rx = self.confirmation_rx.lock().await;
                    while let Some((req_id, tool_confirmation)) = rx.recv().await {
                        if req_id == request.id {
                            let confirmed = tool_confirmation.permission == Permission::AllowOnce|| tool_confirmation.permission == Permission::AlwaysAllow;
                            if confirmed {
                                // Add this tool call to the futures collection
                                let tool_future = self.dispatch_tool_call(tool_call.clone(), request.id.clone());
                                let mut futures = tool_futures.lock().await;
                                futures.push(Box::pin(tool_future));

                                if tool_confirmation.permission == Permission::AlwaysAllow {
                                    permission_manager.update_user_permission(&tool_call.name, PermissionLevel::AlwaysAllow);
                                }
                            } else {
                                // User declined - add declined response
                                let denied_content_text = Content::text(
                                    "The user has declined to run this tool. \
                                    DO NOT attempt to call this tool again. \
                                    If there are no alternative methods to proceed, clearly explain the situation and STOP.");
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![denied_content_text.clone()]),
                                );
                            }
                            break; // Exit the loop once the matching `req_id` is found
                        }
                    }
                }
            }
        }
        .boxed()
    }

    pub(crate) fn handle_frontend_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        message_tool_response: Arc<Mutex<Message>>,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            for request in tool_requests {
                if let Ok(tool_call) = request.tool_call.clone() {
                    if self.is_frontend_tool(&tool_call.name) {
                        // Send frontend tool request and wait for response
                        yield Message::assistant().with_frontend_tool_request(
                            request.id.clone(),
                            Ok(tool_call.clone())
                        );

                        if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                            let mut response = message_tool_response.lock().await;
                            *response = response.clone().with_tool_response(id, result);
                        }
                    }
                }
            }
        }
        .boxed()
    }

    pub(crate) fn handle_enable_extension_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        install_results: ExtensionInstallResults,
        message_tool_response: Arc<Mutex<Message>>,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        let denied_content_text = Content::text(
                                "The user has declined to run this tool. \
                                DO NOT attempt to call this tool again. \
                                If there are no alternative methods to proceed, clearly explain the situation and STOP.");
        try_stream! {
            for extension_request in tool_requests {
                if let Ok(tool_call) = extension_request.tool_call.clone() {
                    let confirmation = Message::user().with_enable_extension_request(
                        extension_request.id.clone(),
                        tool_call
                            .arguments
                            .get("extension_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );

                    yield confirmation;

                    let mut rx = self.confirmation_rx.lock().await;
                    while let Some((req_id, extension_confirmation)) = rx.recv().await {
                        if req_id == extension_request.id {
                            if extension_confirmation.permission == Permission::AllowOnce || extension_confirmation.permission == Permission::AlwaysAllow {
                                let extension_name = tool_call
                                    .arguments
                                    .get("extension_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                let mut results = install_results.lock().await;

                                let install_result = self.enable_extension(
                                    extension_name.clone(),
                                    extension_request.id.clone(),
                                ).await;

                                results.push(install_result);
                            } else {
                                // User declined - add declined response
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    extension_request.id.clone(),
                                    Ok(vec![denied_content_text.clone()]),
                                );
                            }
                            break; // Exit the loop once the matching `req_id` is found
                        }
                    }
                }
            }
        }
        .boxed()
    }

    pub(crate) async fn handle_search_extension_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        message_tool_response: Arc<Mutex<Message>>,
    ) {
        let extension_manager = self.extension_manager.lock().await;
        let mut tool_futures = Vec::new();

        for search_request in tool_requests {
            if let Ok(tool_call) = search_request.tool_call.clone() {
                let is_frontend_tool = self.is_frontend_tool(&tool_call.name);

                let tool_future = Self::create_tool_future(
                    &extension_manager,
                    tool_call,
                    is_frontend_tool,
                    search_request.id.clone(),
                );

                tool_futures.push(tool_future);
            }
        }

        // Wait for all tool calls to complete
        let results = futures::future::join_all(tool_futures).await;
        for (request_id, output) in results {
            let mut response = message_tool_response.lock().await;
            *response = response.clone().with_tool_response(request_id, output);
        }
    }

    /// Create a future that will execute a tool call
    pub(crate) async fn create_tool_future(
        extension_manager: &ExtensionManager,
        tool_call: ToolCall,
        is_frontend_tool: bool,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let result = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            extension_manager
                .read_resource(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            extension_manager
                .list_resources(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
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
}
