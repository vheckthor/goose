use anyhow::Result;
use goose::message::Message;
use goose::model::ModelConfig;
use goose_llm::{completion, CompletionResponse};
use mcp_core::tool::Tool;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = "databricks";
    let model_name = "gpt-4o-mini";
    let model_config = ModelConfig::new(model_name.to_string());

    // Create a message sequence that includes a tool response with both text and image
    let messages = vec![Message::user().with_text("Add 10037 + 23123")];

    let calculator_tool = Tool::new(
        "calculator",
        "Perform basic arithmetic operations",
        json!({
            "type": "object",
            "required": ["operation", "numbers"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform",
                },
                "numbers": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "List of numbers to operate on in order",
                }
            }
        }),
        None,
    );

    let tools = vec![calculator_tool];

    let system_preamble =
        "You are a helpful assistant that can perform arithmetic operations and view images.";

    let completion_response: CompletionResponse = completion(
        provider,
        model_config,
        system_preamble,
        &messages,
        &tools,
        false,
    )
    .await?;

    // Print the response and usage statistics
    println!("\nCompletion Response:");
    println!("---------------");
    println!("{:?}", completion_response);

    Ok(())
}
