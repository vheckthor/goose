use std::collections::HashSet;

use crate::agents::platform_tools::PLATFORM_ENABLE_EXTENSION_TOOL_NAME;
use crate::agents::platform_tools::PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME;
use crate::agents::Agent;
use crate::config::PermissionManager;
use crate::message::{Message, MessageContent, ToolRequest};
use crate::permission::permission_judge::check_tool_permissions;

impl Agent {
    /// Categorize tool requests from the response into different types
    /// Returns:
    /// - frontend_requests: Tool requests that should be handled by the frontend
    /// - enable_extension_requests: Requests to enable extensions
    /// - search_extension_requests: Requests to search for extensions
    /// - other_requests: All other tool requests
    /// - filtered_message: The original message with tool requests removed
    pub(crate) fn categorize_tool_requests(
        &self,
        response: &Message,
    ) -> (
        Vec<ToolRequest>,
        Vec<ToolRequest>,
        Vec<ToolRequest>,
        Vec<ToolRequest>,
        Message,
    ) {
        // First collect any tool requests
        let tool_requests: Vec<ToolRequest> = response
            .content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    Some(req.clone())
                } else {
                    None
                }
            })
            .collect();

        // Create a filtered message with tool requests removed
        let filtered_content = response
            .content
            .iter()
            .filter(|c| !matches!(c, MessageContent::ToolRequest(_)))
            .cloned()
            .collect();

        let filtered_message = Message {
            role: response.role.clone(),
            created: response.created,
            content: filtered_content,
        };

        // Categorize tool requests
        let mut frontend_requests = Vec::new();
        let mut enable_extension_requests = Vec::new();
        let mut search_extension_requests = Vec::new();
        let mut other_requests = Vec::new();

        for request in tool_requests {
            if let Ok(tool_call) = &request.tool_call {
                if self.is_frontend_tool(&tool_call.name) {
                    frontend_requests.push(request);
                } else if tool_call.name == PLATFORM_ENABLE_EXTENSION_TOOL_NAME {
                    enable_extension_requests.push(request);
                } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
                    search_extension_requests.push(request);
                } else {
                    other_requests.push(request);
                }
            } else {
                // If there's an error in the tool call, add it to other_requests
                other_requests.push(request);
            }
        }

        (
            frontend_requests,
            enable_extension_requests,
            search_extension_requests,
            other_requests,
            filtered_message,
        )
    }

    pub(crate) async fn categorize_regular_tool_requests(
        &self,
        tool_requests: &[ToolRequest],
        mode: &str,
        tools_with_readonly_annotation: HashSet<String>,
        tools_without_annotation: HashSet<String>,
    ) -> (Vec<ToolRequest>, Vec<ToolRequest>, Vec<ToolRequest>) {
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

        (
            permission_check_result.approved,
            permission_check_result.denied,
            permission_check_result.needs_approval,
        )
    }
}
