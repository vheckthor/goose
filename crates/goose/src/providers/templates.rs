use std::collections::HashMap;
use serde_json::{Value, json};
use regex::Regex;

use mcp_core::role::Role;
use mcp_core::{Tool, ToolCall};
use crate::message::MessageContent;

#[derive(Clone, serde::Serialize)]
pub struct TemplatedToolConfig {
    pub stop_tokens: Vec<String>,
}

impl TemplatedToolConfig {
    pub fn deepseek_style() -> Self {
        Self {
            stop_tokens: vec![],  // No special stop tokens needed anymore
        }
    }
}

pub struct TemplateContext<'a> {
    pub system: Option<&'a str>,
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
                output.push_str("Available tools:\n");
                for tool in tools {
                    // Create the desired schema format
                    let desired_schema = json!({
                        "name": {
                            "type": "string"
                        },
                        "parameters": tool.input_schema,
                        "required": ["name", "parameters"]
                    });
                    output.push_str(&format!("- Tool name: {}\nTool description: {}\nTool input schema: {}\n", tool.name, tool.description, desired_schema));
                }
                output.push_str("\nTo use a tool, respond with a JSON object with 'name' and 'parameters' fields.\n\n");
                output.push_str("Only use tools when needed. For general questions, respond directly without using tools.\n\n");
            }
        }

        output
    }

    pub fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        use std::collections::HashSet;
        let mut seen_calls = HashSet::new();
        let mut tool_calls = Vec::new();
        
        // Find all matches of the tool call pattern in the text
        for cap in self.tool_call_regex.find_iter(response) {
            let tool_call_str = cap.as_str();
            if let Ok(parsed) = serde_json::from_str::<Value>(tool_call_str) {
                if let (Some(name), Some(parameters)) = (
                    parsed.get("name").and_then(|n| n.as_str()),
                    parsed.get("parameters"),
                ) {
                    // Create a string that uniquely identifies this tool call
                    let tool_call_key = format!("{}:{}", name, parameters.to_string());
                    
                    // Only add if we haven't seen this exact tool call before
                    if seen_calls.insert(tool_call_key) {
                        tool_calls.push(ToolCall::new(name, parameters.clone()));
                    }
                }
            }
        }
        
        tool_calls
    }
}