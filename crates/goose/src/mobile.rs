use std::collections::HashMap;
use anyhow::Result as AnyhowResult;
use crate::errors::{AgentError, AgentResult};
use async_trait::async_trait;
use base64::prelude::*;
use indoc::{formatdoc, indoc};
use serde_json::{json, Value};
use adb_client::{ADBServer, ADBDeviceExt};

use crate::models::tool::{Tool, ToolCall};
use crate::models::content::Content;
use crate::systems::System;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MobileSystem provides functionality for instrumenting and controlling Android devices
/// either via USB connection or through an emulator.
pub struct MobileSystem {
    android_tool: Tool,
    instructions: String,
    device: Option<Arc<RwLock<adb_client::ADBServerDevice>>>,
    screen_size: Option<(i32, i32)>,
}

impl Default for MobileSystem {
    fn default() -> Self {
        Self::new()
    }
}


impl MobileSystem {
    pub fn new() -> Self {
        let android_tool = Tool::new(
            "android",
            indoc! {r#"
                Interact with an Android device or emulator.
                  - You can send clicks, input text, and capture screenshots.
                  - Send zero or more commands to the device.
                  - Receive a screenshot of the device after each command.

                For example to send a text message, you'd find the text message app first, then start it, then enter
                the phone number, then enter the message, then send it.
            "#},
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "enum": ["home", "click", "enter_text", "screenshot"],
                        "description": "The commands to run."
                    },
                    "click_where": {
                        "type": "object",
                        "properties": {
                            "x": {
                                "type": "integer",
                                "default": null,
                                "description": "X coordinate to click."
                            },
                            "y": {
                                "type": "integer",
                                "default": null,
                                "description": "Y coordinate to click."
                            }
                        },
                        "required": ["x", "y"]
                    },
                    "enter_text": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "default": null,
                                "description": "Text to enter."
                            }
                        },
                        "required": ["text"]
                    }
                },
                "required": ["command"]
            }),
        );
        let instructions= formatdoc! {r#"
            To use the mobile system, you need to have an Android device or emulator connected to your computer.
            You can use the following tools to interact with the device:

            - `android`: Interact with an Android device or emulator.
              - You can press home, send clicks, input text, and capture screenshots.
              - Send zero or more commands to the device.
              - Receive an xml dump of the UI hierarchy of the current Android app after each command.

            For example to send a text message, you'd find the text message app first, then start it, then enter
            the phone number, then enter the message, then send it.
            "#};

            let (device, screen_size) = match MobileSystem::initialize_device() {
                Ok((device, screen_size)) => (Some(Arc::new(RwLock::new(device))), Some(screen_size)),
                Err(_) => (None, None),
            };
        
        Self {
            android_tool,
            instructions,
            device,
            screen_size,
        }
    }

    fn get_argument<'a>(
        &self,
        tool_call: &'a ToolCall,
        key1: &str,
        key2: Option<&str>,
    ) -> Option<&'a Value> {
        let value = tool_call.arguments.get(key1)?;
        match key2 {
            Some(sub_key) => value.get(sub_key),
            None => Some(value),
        }
    }

    async fn run_shell_command(
        &self,
        command: &[&str],
    ) -> Result<Vec<u8>, AgentError> {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| AgentError::ExecutionError("Device not connected.".to_string()))?;

        let mut device = device.write().await;
        let mut output = Vec::new();

        device
            .shell_command(command, &mut output)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to run command {:?}: {}", command, e)))?;

        Ok(output)
    }

    fn initialize_device() -> Result<(adb_client::ADBServerDevice, (i32, i32)), AgentError> {
        let mut server = ADBServer::default();
        let mut device = server
            .get_device()
            .map_err(|e| AgentError::ExecutionError(format!("Failed to connect to device: {}", e)))?;

        // Fetch screen size
        let mut output = Vec::new();
        device
            .shell_command(["wm", "size"], &mut output)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to get screen size: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output);
        let size = output_str
            .lines()
            .find(|line| line.contains("Physical size"))
            .and_then(|line| {
                line.split_whitespace()
                    .nth(2) // Get the size part
                    .map(|dim| {
                        let mut parts = dim.split('x');
                        (
                            parts.next().unwrap_or("0").parse::<i32>().unwrap_or(0),
                            parts.next().unwrap_or("0").parse::<i32>().unwrap_or(0),
                        )
                    })
            })
            .ok_or_else(|| AgentError::ExecutionError("Failed to parse screen size.".to_string()))?;

        Ok((device, size))
    }


}


#[async_trait]
impl System for MobileSystem {
    fn name(&self) -> &str {
        "MobileSystem"
    }

    fn description(&self) -> &str {
        "System to manage a mobile device or emulator."
    }

    fn instructions(&self) -> &str {
        self.instructions.as_str()
    }

    fn tools(&self) -> &[Tool] {
        if self.device.is_some() {
            std::slice::from_ref(&self.android_tool)
        } else {
            &[]
        }
    }

    async fn status(&self) -> AnyhowResult<HashMap<String, Value>> {
        let mut status = HashMap::new();
        status.insert(
            "connected".to_string(),
            json!(self.device.is_some()),
        );

        if let Some((width, height)) = self.screen_size {
            status.insert("screen_size".to_string(), json!({ "width": width, "height": height }));
        }

        Ok(status)
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match tool_call.name.as_str() {
            "android" => {
                let response_message = match self.get_argument(&tool_call, "command", None).and_then(Value::as_str) {
                    Some("home") => {
                        self.run_shell_command(&["input", "keyevent", "KEYCODE_HOME"]).await?;
                        Content::text("Sent home key event.".to_string())
                    }
                    Some("click") => {
                        let x = self.get_argument(&tool_call, "click_where", Some("x")).and_then(Value::as_i64);
                        let y = self.get_argument(&tool_call, "click_where", Some("y")).and_then(Value::as_i64);
    
                        match (x, y) {
                            (Some(x), Some(y)) => {
                                let command = format!("input tap {} {}", x, y);
                                self.run_shell_command(&command.split_whitespace().collect::<Vec<&str>>()).await?;
                                Content::text(format!("Clicked at coordinates ({}, {}).", x, y))
                            }
                            _ => {
                                return Err(AgentError::ExecutionError("Missing or invalid click coordinates.".to_string()))
                            }
                        }
                    }
                    Some("enter_text") => {
                        let text = self.get_argument(&tool_call, "enter_text", Some("text")).and_then(Value::as_str);
    
                        if let Some(text) = text {
                            let command = format!("input text '{}'", text);
                            self.run_shell_command(&command.split_whitespace().collect::<Vec<&str>>()).await?;
                            Content::text(format!("Entered text: '{}'.", text))
                        } else {
                            return Err(AgentError::ExecutionError("Missing or invalid text input.".to_string()));
                        }
                    }
                    Some("screenshot") => {
                        let screenshot_data = self.run_shell_command(&["screencap", "-p"]).await?;
                        std::fs::write("screenshot.png", &screenshot_data).map_err(|e| {
                            AgentError::ExecutionError(format!("Failed to write screenshot: {}", e))
                        })?;
    
                        let image_data = BASE64_STANDARD.encode(&screenshot_data);
                        Content::image(image_data, "image/png")
                    }
                    _ => return Err(AgentError::ExecutionError("Invalid or unsupported command.".to_string())),
                };
    
                // Always include the UI hierarchy XML in the response
                self.run_shell_command(&["uiautomator", "dump", "/sdcard/window_dump.xml"]).await?;
    
                let xml_content = self.run_shell_command(&["cat", "/sdcard/window_dump.xml"]).await?;
                let ui_dump = String::from_utf8(xml_content)
                    .map_err(|e| AgentError::ExecutionError(format!("Failed to parse XML content: {}", e)))?;

                std::fs::write("uihierarchy.xml", &ui_dump).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write UI hierarchy: {}", e))
                })?;

                Ok(vec![response_message, Content::text(ui_dump)])

            }
            _ => Err(AgentError::ExecutionError("Unknown tool name.".to_string())),
        }
    }
    
    
}
