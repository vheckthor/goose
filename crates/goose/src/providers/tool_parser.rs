use anyhow::Result;
use serde_json::Value;
/// Helper function to parse tool calls from text content
pub fn parse_tool_calls_from_text(content: &str) -> Result<Vec<Value>> {
    println!("\n=== Tool Parser Debug ===");
    println!("Input content:\n{}", content);

    // First try to parse the content directly as JSON
    println!("Attempting direct JSON parse...");
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        // Check if it's a valid tool call format
        if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
            if tool.is_string() && args.is_object() {
                println!("Successfully parsed direct JSON tool call");
                return Ok(vec![json]);
            }
        }
        // Check if it's an array of tool calls
        if let Some(array) = json.as_array() {
            if array.iter().all(|item| {
                item.get("tool").map_or(false, |t| t.is_string()) &&
                item.get("args").map_or(false, |a| a.is_object())
            }) {
                println!("Successfully parsed JSON array of tool calls");
                return Ok(array.to_vec());
            }
        }
        println!("JSON parsed but not in tool call format");
    } else {
        println!("Direct JSON parse failed");
    }

    println!("=== End Tool Parser Debug ===\n");
    Ok(vec![])
}

/// A lightweight provider specifically for parsing tool calls
#[derive(serde::Serialize, Default)]
pub struct ToolParserProvider;

impl ToolParserProvider {
    pub async fn parse_tool_calls(&self, content: &str) -> Result<Vec<Value>> {
        parse_tool_calls_from_text(content)
    }
}
