use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const ROUTER_VECTOR_SEARCH_TOOL_NAME: &str = "router__vector_search";
pub const ROUTER_ACTIVATE_TOOL_NAME: &str = "router__activate_tool";

pub fn vector_search_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_TOOL_NAME.to_string(),
        indoc! {r#"
            Search for tools based on the user's input.
            This tool searches for the most relevant tools based on the user's input.
            It uses a vector database to find the most relevant tools.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "input": {"type": "string", "description": "The user's input"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Vector search for relevant tools".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn activate_tool() -> Tool {
    Tool::new(
        ROUTER_ACTIVATE_TOOL_NAME.to_string(),
        indoc! {r#"
            Activate a tool depending on whether the tool was selected by the vector search or not.
            If the tool was selected by the vector search, this tool will activate the tool.
            If the tool was not selected by the vector search, this tool will not activate the tool.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["tool_name"],
            "properties": {
                "tool_name": {"type": "string", "description": "The name of the tool to activate"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Activate a tool".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}
