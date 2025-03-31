# DeepSeek Tool Calling Implementation

This implementation adds support for tool calling with DeepSeek models by embedding tools directly in the system prompt, as shown in the Hugging Face Rust bindings example.

## Key Components

### 1. DeepSeek Model Detection

```rust
fn is_deepseek_model(&self) -> bool {
    self.model.model_name.contains("deepseek") || 
    self.model.model_name.contains("DeepSeek")
}
```

This function detects when we're working with a DeepSeek model.

### 2. Tool Embedding in System Prompt

```rust
fn create_system_prompt_with_tools(&self, system: &str, tools: &[Tool]) -> String {
    if tools.is_empty() {
        return system.to_string();
    }
    
    // Start with the original system prompt
    let mut tool_system_prompt = format!("{}\n\n## Tools\n", system);
    
    // Add function section
    tool_system_prompt.push_str("\n### Function\n\n");
    tool_system_prompt.push_str("You have the following functions available:\n\n");
    
    // Add each tool as a function definition in the format shown in the example
    for tool in tools {
        tool_system_prompt.push_str(&format!("- `{}`:\n```json\n{}\n```\n\n", 
            tool.name,
            json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema
            }).to_string()
        ));
    }
    
    tool_system_prompt
}
```

This function embeds tools directly in the system prompt following the format shown in the Hugging Face example.

### 3. Updated Complete Method

```rust
async fn complete(
    &self,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Message, ProviderUsage), ProviderError> {
    // For DeepSeek models, embed tools in the system prompt
    let system_prompt = if self.is_deepseek_model() && !tools.is_empty() {
        self.create_system_prompt_with_tools(system, tools)
    } else {
        system.to_string()
    };
    
    // Create request with the appropriate system prompt and tools
    let payload = create_request(
        &self.model, 
        &system_prompt, 
        messages, 
        tools, 
        &ImageFormat::OpenAi
    )?;
    
    // Make the request
    let response = self.post(payload.clone()).await?;
    
    // Parse response
    let message = response_to_message(response.clone())?;
    let usage = match get_usage(&response) {
        Ok(usage) => usage,
        Err(ProviderError::UsageError(e)) => {
            tracing::debug!("Failed to get usage data: {}", e);
            Usage::default()
        }
        Err(e) => return Err(e),
    };
    let model = get_model(&response);
    emit_debug_trace(&self.model, &payload, &response, &usage);
    Ok((message, ProviderUsage::new(model, usage)))
}
```

The `complete` method now checks if we're using a DeepSeek model and, if so, embeds the tools in the system prompt.

## Implementation Notes

1. For DeepSeek models, tools are embedded directly in the system prompt following the exact format from the Hugging Face example.
2. For other models, the standard OpenAI-compatible API is used.
3. The implementation is minimal and focused, adding only what's necessary to support tool calling with DeepSeek models.
4. The code compiles without warnings.