use std::collections::HashMap;
use serde_json::Value;
use regex::Regex;

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
    #[serde(skip)]
    tool_call_regex: Regex,
}

impl TemplateRenderer {
    pub fn new(config: TemplatedToolConfig) -> Self {
        // This regex looks for JSON objects that have a "name" and "parameters" field
        // It's permissive about whitespace and allows for any valid JSON in the parameters
        let tool_call_regex = Regex::new(
            r#"\{[\s\n]*"name"[\s\n]*:[\s\n]*"[^"]+",[\s\n]*"parameters"[\s\n]*:[\s\n]*\{[^}]*\}[\s\n]*\}"#
        ).expect("Failed to compile tool call regex");

        Self {
            config,
            tool_call_regex,
        }
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
                output.push_str("Tool calls should be formatted as JSON objects with 'name' and 'parameters' fields.\n");
                output.push_str("Example:\n");
                output.push_str(r#"{"name": "tool_name", "parameters": {"param1": "value1"}}"#);
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
                        for content in &message.content {
                            if let MessageContent::ToolRequest(request) = content {
                                if let Ok(tool_call) = &request.tool_call {
                                    output.push_str(&format!(
                                        r#"{{"name": "{}", "parameters": {}}}"#,
                                        tool_call.name, tool_call.arguments
                                    ));
                                    output.push_str("\n");
                                }
                            }
                        }
                    } else {
                        output.push_str(&message.as_concat_text());
                        output.push_str("\n");
                    }
                }
                Role::Tool => {
                    output.push_str(&message.as_concat_text());
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
        
        // Find all matches of the tool call pattern in the text
        for cap in self.tool_call_regex.find_iter(response) {
            let tool_call_str = cap.as_str();
            if let Ok(parsed) = serde_json::from_str::<Value>(tool_call_str) {
                if let (Some(name), Some(parameters)) = (
                    parsed.get("name").and_then(|n| n.as_str()),
                    parsed.get("parameters"),
                ) {
                    tool_calls.push(ToolCall::new(name, parameters.clone()));
                }
            }
        }
        
        tool_calls
    }
}