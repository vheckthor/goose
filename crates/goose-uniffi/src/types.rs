use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use serde_json::ser::Serializer;
use serde_json::de::Deserializer;

use crate::tool_result_serde;

#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq, uniffi::Error)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Schema error: {0}")]
    SchemaError(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

pub type ToolResult<T> = std::result::Result<T, ToolError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    ToolReq(ToolRequest),
    ToolResp(ToolResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    pub tool_call: ToolRequestToolCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    pub tool_result: ToolResponseToolResult
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub created: i64,
    pub content: Vec<MessageContent>,
}


// — Newtype wrappers (local structs) so we satisfy Rust’s orphan rules —
// We need these because we can’t implement UniFFI’s FfiConverter directly on a type alias.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRequestToolCall(pub ToolResult<ToolCall>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponseToolResult(pub ToolResult<Vec<TextContent>>);

// — Register the newtypes with UniFFI, converting via JSON strings —
// UniFFI’s FFI layer supports only primitive buffers (here String), so we JSON-serialize
// through our `tool_result_serde` to preserve the same success/error schema on both sides.

uniffi::custom_type!(ToolRequestToolCall, String, {
    lower: |wrapper: &ToolRequestToolCall| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            // note the borrow on wrapper.0
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolRequestToolCall serialization failed");
        }
        String::from_utf8(buf).expect("ToolRequestToolCall produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolRequestToolCall(result))
    },
});

uniffi::custom_type!(ToolResponseToolResult, String, {
    lower: |wrapper: &ToolResponseToolResult| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolResponseToolResult serialization failed");
        }
        String::from_utf8(buf).expect("ToolResponseToolResult produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolResponseToolResult(result))
    },
});