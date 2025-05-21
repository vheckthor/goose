use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const ROUTER_VECTOR_SEARCH_TOOL_NAME: &str = "router__vector_search";

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
