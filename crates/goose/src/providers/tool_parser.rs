use anyhow::{Result, Context};
use serde_json::Value;
use reqwest::Client;
use std::time::Duration;
use url::Url;
use regex::Regex;
use mcp_core::tool::Tool;

const OLLAMA_HOST: &str = "localhost";
const OLLAMA_PORT: u16 = 11434;
const OLLAMA_MODEL: &str = "llama3.2";

async fn parse_with_ollama(content: &str, tools: &[Tool]) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()?;

    let base_url = format!("http://{}:{}", OLLAMA_HOST, OLLAMA_PORT);
    let url = Url::parse(&base_url)?
        .join("v1/chat/completions")?;
    // println!("Ollama URL: {}", url);

    let mut system = "You are a JSON parser that finds malformed JSON and converts it into valid JSON.".to_string();
    
    // if !tools.is_empty() {
    //     system.push_str("\nTool definitions: ");
    //     let tools_text = serde_json::to_string_pretty(&tools)
    //         .unwrap_or_else(|_| "[Error serializing tools]".to_string());
    //     system.push_str(&tools_text);
    // }

    let prompt = format!(
        "Analyze the following text. If there are references to valid available tools, help rewrite the proper, valid JSON for the tool calls. \
        The JSON must be EXACTLY in this format, with no deviations:\n\
        {{\n\
          \"tool\": \"tool_name\",     // Must be a string, not an array or object\n\
          \"args\": {{                 // Must be a direct object at this level\n\
            \"param1\": \"value1\",    // Tool-specific parameters\n\
            \"param2\": \"value2\"\n\
          }}\n\
        }}\n\
        For example:\n\
        {{\n\
          \"tool\": \"developer__shell\",\n\
          \"args\": {{\n\
            \"command\": \"ls -l\"\n\
          }}\n\
        }}\n\
        If multiple tool calls are found, return an array of objects in this exact format.\n\
        Only output the JSON, no other text. If no reference to these tools is found, return N/A.\n\n{}", 
        content
    );

    let payload = serde_json::json!({
        "model": OLLAMA_MODEL,
        "messages": [
            {
                "role": "system",
                "content": system
            },
            {
                "role": "user", 
                "content": prompt
            }
        ],
        "stream": false
    });

    // println!("Ollama request payload: {}", serde_json::to_string_pretty(&payload)?);

    // println!("Sending request to Ollama...");
    let response = match client.post(url)
        .json(&payload)
        .send()
        .await {
            Ok(r) => {
                // println!("Successfully connected to Ollama");
                r
            },
            Err(e) => {
                println!("Failed to connect to Ollama: {}", e);
                return Err(e.into());
            }
        };

    // println!("Ollama response status: {}", response.status());
    let response_text = response.text().await?;
    // println!("Ollama raw response: {}", response_text);

    let json: Value = serde_json::from_str(&response_text)?;

    // println!("Ollama parsed JSON: {}", serde_json::to_string_pretty(&json)?);

    let content = json.get("choices")
        .and_then(|choices| {
            // println!("Found choices: {}", serde_json::to_string_pretty(choices).unwrap_or_default());
            choices.get(0)
        })
        .and_then(|choice| {
            // println!("Found first choice: {}", serde_json::to_string_pretty(choice).unwrap_or_default());
            choice.get("message")
        })
        .and_then(|message| {
            // println!("Found message: {}", serde_json::to_string_pretty(message).unwrap_or_default());
            message.get("content")
        })
        .and_then(|content| {
            // println!("Found content: {}", serde_json::to_string_pretty(content).unwrap_or_default());
            content.as_str()
        })
        .context("No json from Ollama response")?;

    Ok(content.to_string())
}

/// Extract JSON from markdown-style code blocks
fn extract_json_from_codeblocks(content: &str) -> Vec<String> {
    // Match ```json blocks with or without newlines, and optional language specification
    let re = Regex::new(r"```(?:json)?\s*([\s\S]*?)\s*```").unwrap();
    re.captures_iter(content)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .collect()
}

/// Helper function to parse tool calls from text content
pub async fn parse_tool_calls_from_text(content: &str, tools: &[Tool]) -> Result<Vec<Value>> {
    // println!("\n=== Tool Parser Debug ===");
    // println!("Input content:\n{}", content);

    // First check for JSON code blocks
    // println!("Checking for JSON code blocks...");
    let code_blocks = extract_json_from_codeblocks(content);
    for json_str in code_blocks {
        // println!("Found JSON code block:\n{}", json_str);
        if let Ok(json) = serde_json::from_str::<Value>(&json_str) {
            // println!("Successfully parsed code block as JSON: {}", serde_json::to_string_pretty(&json)?);
            // Check if it's a valid tool call format
            if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
                if tool.is_string() && args.is_object() {
                    // println!("Successfully parsed tool call from code block");
                    return Ok(vec![json]);
                }
            }
            // println!("Code block parsed as JSON but not in tool call format");
        } 
        // else {
        //     println!("Failed to parse code block as JSON");
        // }
    }

    // Then try to parse the content directly as JSON
    // println!("Attempting direct JSON parse...");
    // println!("Attempting to parse: {}", content);
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        // println!("Successfully parsed as JSON: {}", serde_json::to_string_pretty(&json)?);
        // Check if it's a valid tool call format
        if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
            if tool.is_string() && args.is_object() {
                // println!("Successfully parsed direct JSON tool call");
                return Ok(vec![json]);
            }
        }
        // Check if it's an array of tool calls
        if let Some(array) = json.as_array() {
            if array.iter().all(|item| {
                item.get("tool").map_or(false, |t| t.is_string()) &&
                item.get("args").map_or(false, |a| a.is_object())
            }) {
                // println!("Successfully parsed JSON array of tool calls");
                return Ok(array.to_vec());
            }
        }
        // println!("JSON parsed but not in tool call format");
    } 
    // else {
    //     println!("Failed to parse content as JSON");
    // }

    // Try using ollama as fallback
    // println!("Attempting Ollama parse...");
    if let Ok(json_str) = parse_with_ollama(content, tools).await {
        // println!("Got response from Ollama: {}", json_str);
        if let Ok(json) = serde_json::from_str::<Value>(&json_str) {
            // println!("Successfully parsed Ollama response as JSON: {}", serde_json::to_string_pretty(&json)?);
            // Check if it's a valid tool call format
            if let (Some(tool), Some(args)) = (json.get("tool"), json.get("args")) {
                if tool.is_string() && args.is_object() {
                    // println!("Successfully parsed tool call using Ollama");
                    return Ok(vec![json]);
                }
            }
            // Check if it's an array of tool calls
            if let Some(array) = json.as_array() {
                if array.iter().all(|item| {
                    item.get("tool").map_or(false, |t| t.is_string()) &&
                    item.get("args").map_or(false, |a| a.is_object())
                }) {
                    // println!("Successfully parsed array of tool calls using Ollama");
                    return Ok(array.to_vec());
                }
            }
            // println!("Ollama output parsed as JSON but not in tool call format");
        } else {
            // println!("Failed to parse Ollama output as JSON");
        }
    } else {
        // println!("Ollama parse failed");
    }

    // println!("=== End Tool Parser Debug ===\n");
    Ok(vec![])
}

/// A lightweight provider specifically for parsing tool calls
#[derive(serde::Serialize, Default)]
pub struct ToolParserProvider;

impl ToolParserProvider {
    pub async fn parse_tool_calls(&self, content: &str, tools: &[Tool]) -> Result<Vec<Value>> {
        parse_tool_calls_from_text(content, tools).await
    }
}
