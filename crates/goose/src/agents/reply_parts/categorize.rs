use std::collections::HashSet;

use mcp_core::Tool;

use crate::agents::platform_tools::PLATFORM_ENABLE_EXTENSION_TOOL_NAME;
use crate::agents::platform_tools::PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME;
use crate::agents::Agent;
use crate::message::{Message, MessageContent, ToolRequest};

impl Agent {
    /// Categorize tools based on their annotations
    /// Returns:
    /// - read_only_tools: Tools with read-only annotations
    /// - non_read_tools: Tools without read-only annotations
    pub(crate) fn categorize_tools_by_annotation(
        tools: &[Tool],
    ) -> (HashSet<String>, HashSet<String>) {
        tools
            .iter()
            .fold((HashSet::new(), HashSet::new()), |mut acc, tool| {
                match &tool.annotations {
                    Some(annotations) if annotations.read_only_hint => {
                        acc.0.insert(tool.name.clone());
                    }
                    _ => {
                        acc.1.insert(tool.name.clone());
                    }
                }
                acc
            })
    }

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
}
