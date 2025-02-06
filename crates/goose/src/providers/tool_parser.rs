use anyhow::{Result, Context};
use serde_json::Value;
use reqwest::Client;
use std::time::Duration;
use url::Url;

const OLLAMA_HOST: &str = "localhost";
const OLLAMA_PORT: u16 = 11434;
const OLLAMA_MODEL: &str = "llama3.2";

async fn parse_with_ollama(content: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()?;

    let base_url = format!("http://{}:{}", OLLAMA_HOST, OLLAMA_PORT);
    let url = Url::parse(&base_url)?
        .join("v1/chat/completions")?;

    let prompt = format!(
        "Parse the following text into a JSON object with 'tool' and 'args' fields. \
        The 'tool' should be a string and 'args' should be an object. \
        If multiple tool calls are found, return an array of such objects. \
        Here's the text to parse:\n\n{}", 
        content
    );

    let payload = serde_json::json!({
        "model": OLLAMA_MODEL,
        "messages": [
            {
                "role": "system",
                "content": "You are a JSON parser that converts text into tool call format."
            },
            {
                "role": "user", 
                "content": prompt
            }
        ],
        "stream": false
    });

    let response = client.post(url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send request to Ollama")?;

    let json = response.json::<Value>().await
        .context("Failed to parse Ollama response as JSON")?;

    let content = json.get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .context("Failed to extract content from Ollama response")?;

    Ok(content.to_string())
}
/// Helper function to parse tool calls from text content
pub async fn parse_tool_calls_from_text(content: &str) -> Result<Vec<Value>> {
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

    // Try using ollama as fallback
    println!("Attempting Ollama parse...");
    if let Ok(json_str) = parse_with_ollama(content).await {
        if let Ok(json) = serde_json::from_str::<Value>(&json_str) {
            // Check if it's a valid tool call format
            if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
                if tool.is_string() && args.is_object() {
                    println!("Successfully parsed tool call using Ollama");
                    return Ok(vec![json]);
                }
            }
            // Check if it's an array of tool calls
            if let Some(array) = json.as_array() {
                if array.iter().all(|item| {
                    item.get("tool").map_or(false, |t| t.is_string()) &&
                    item.get("args").map_or(false, |a| a.is_object())
                }) {
                    println!("Successfully parsed array of tool calls using Ollama");
                    return Ok(array.to_vec());
                }
            }
            println!("Ollama output parsed as JSON but not in tool call format");
        } else {
            println!("Failed to parse Ollama output as JSON");
        }
    } else {
        println!("Ollama parse failed");
    }

    println!("=== End Tool Parser Debug ===\n");
    Ok(vec![])
}

/// A lightweight provider specifically for parsing tool calls
#[derive(serde::Serialize, Default)]
pub struct ToolParserProvider;

impl ToolParserProvider {
    pub async fn parse_tool_calls(&self, content: &str) -> Result<Vec<Value>> {
        parse_tool_calls_from_text(content).await
    }
}
