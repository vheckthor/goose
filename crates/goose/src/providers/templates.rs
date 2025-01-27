use std::collections::HashMap;
use serde_json::Value;

use mcp_core::role::Role;
use mcp_core::{Tool, ToolCall};
use crate::message::MessageContent;

#[derive(Clone, serde::Serialize)]
pub struct TemplatedToolConfig {
    pub template: String,
    pub start_delimiters: Vec<String>,
    pub end_delimiters: Vec<String>,
    pub tool_call_start: String,
    pub tool_call_end: String,
    pub tool_output_start: String,
    pub tool_output_end: String,
    pub stop_tokens: Vec<String>,
    pub role_markers: HashMap<Role, String>,
}

impl TemplatedToolConfig {
    pub fn deepseek_style() -> Self {
        let mut role_markers = HashMap::new();
        role_markers.insert(Role::User, "<｜User｜>".to_string());
        role_markers.insert(Role::Assistant, "<｜Assistant｜>".to_string());
        role_markers.insert(Role::Tool, "".to_string());

        Self {
            template: include_str!("templates/deepseek.txt").to_string(),
            start_delimiters: vec!["<｜tool▁calls▁begin｜>".to_string()],
            end_delimiters: vec!["<｜tool▁calls▁end｜>".to_string()],
            tool_call_start: "<｜tool▁call▁begin｜>".to_string(),
            tool_call_end: "<｜tool▁call▁end｜>".to_string(),
            tool_output_start: "<｜tool▁output▁begin｜>".to_string(),
            tool_output_end: "<｜tool▁output▁end｜>".to_string(),
            stop_tokens: vec![
                "<｜begin▁of▁sentence｜>".to_string(),
                "<｜end▁of▁sentence｜>".to_string(),
                "<｜User｜>".to_string(),
                "<｜Assistant｜>".to_string(),
            ],
            role_markers,
        }
    }
}

pub struct TemplateContext<'a> {
    pub system: Option<&'a str>,
    pub messages: &'a [crate::message::Message],
    pub tools: Option<&'a [Tool]>,
}

#[derive(Clone, serde::Serialize)]
pub struct TemplateRenderer {
    config: TemplatedToolConfig,
}

impl TemplateRenderer {
    pub fn new(config: TemplatedToolConfig) -> Self {
        Self { config }
    }

    pub fn get_stop_tokens(&self) -> &[String] {
        &self.config.stop_tokens
    }

    pub fn render(&self, context: TemplateContext) -> String {
        let mut output = String::new();

        // Add system message if present
        if let Some(system) = context.system {
            output.push_str(system);
            output.push_str("\n\n");
        }

        // Add tools if present
        if let Some(tools) = context.tools {
            if !tools.is_empty() {
                output.push_str("The following tools are available when needed for specific tasks:\n");
                for tool in tools {
                    output.push_str(&format!("- {}: {}\n", tool.name, tool.description));
                }
                output.push_str("\nOnly use tools when the task specifically requires their functionality.\n");
                output.push_str("For general questions or tasks that don't need external data, respond directly.\n\n");
                output.push_str("When using a tool, format as:\n");
                output.push_str(&self.config.start_delimiters[0]);
                output.push_str("\n");
                output.push_str(&self.config.tool_call_start);
                output.push_str("\n{\"name\": \"function_name\", \"parameters\": {\"param1\": \"value1\"}}\n");
                output.push_str(&self.config.tool_call_end);
                output.push_str("\n");
                output.push_str(&self.config.end_delimiters[0]);
                output.push_str("\n\n");
            }
        }

        // Add conversation history
        for message in context.messages {
            match message.role {
                Role::User => {
                    output.push_str(&self.config.role_markers[&Role::User]);
                    output.push_str("\n");
                    output.push_str(&message.as_concat_text());
                    output.push_str("\n");
                }
                Role::Assistant => {
                    output.push_str(&self.config.role_markers[&Role::Assistant]);
                    output.push_str("\n");
                    if message.is_tool_call() {
                        output.push_str(&self.config.start_delimiters[0]);
                        output.push_str("\n");
                        for content in &message.content {
                            if let MessageContent::ToolRequest(req) = content {
                                if let Ok(tool_call) = &req.tool_call {
                                    output.push_str(&self.config.tool_call_start);
                                    output.push_str("\n");
                                    output.push_str(&format!(
                                        "{{\"name\": \"{}\", \"parameters\": {}}}",
                                        tool_call.name, tool_call.arguments
                                    ));
                                    output.push_str("\n");
                                    output.push_str(&self.config.tool_call_end);
                                    output.push_str("\n");
                                }
                            }
                        }
                        output.push_str(&self.config.end_delimiters[0]);
                        output.push_str("\n");
                    } else {
                        output.push_str(&message.as_concat_text());
                        output.push_str("\n");
                    }
                }
                Role::Tool => {
                    output.push_str(&self.config.tool_output_start);
                    output.push_str("\n");
                    output.push_str(&message.as_concat_text());
                    output.push_str("\n");
                    output.push_str(&self.config.tool_output_end);
                    output.push_str("\n");
                }
            }
        }

        // Add final assistant marker for response
        output.push_str(&self.config.role_markers[&Role::Assistant]);
        output.push_str("\n");

        output
    }

    pub fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut tool_calls = Vec::new();
        
        // Find sections between tool call delimiters
        if let Some(tool_section) = self.extract_between_delimiters(response) {
            // Parse individual tool calls
            for call in self.extract_tool_calls(&tool_section) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&call) {
                    if let (Some(name), Some(parameters)) = (
                        parsed.get("name").and_then(|n| n.as_str()),
                        parsed.get("parameters"),
                    ) {
                        tool_calls.push(ToolCall::new(name, parameters.clone()));
                    }
                }
            }
        }
        
        tool_calls
    }

    fn extract_between_delimiters(&self, text: &str) -> Option<String> {
        for start in &self.config.start_delimiters {
            for end in &self.config.end_delimiters {
                if let Some(start_idx) = text.find(start) {
                    if let Some(end_idx) = text[start_idx..].find(end) {
                        return Some(text[start_idx..start_idx + end_idx + end.len()].to_string());
                    }
                }
            }
        }
        None
    }

    fn extract_tool_calls(&self, section: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let mut current_pos = 0;
        
        while let Some(start_idx) = section[current_pos..].find(&self.config.tool_call_start) {
            let start_pos = current_pos + start_idx + self.config.tool_call_start.len();
            if let Some(end_idx) = section[start_pos..].find(&self.config.tool_call_end) {
                let call = section[start_pos..start_pos + end_idx].trim().to_string();
                calls.push(call);
                current_pos = start_pos + end_idx + self.config.tool_call_end.len();
            } else {
                break;
            }
        }
        
        calls
    }
}