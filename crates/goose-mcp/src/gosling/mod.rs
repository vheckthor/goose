use indoc::indoc;
use serde_json::json;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, sync::Mutex};
use tokio::process::Command;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::io::Cursor;
use image;

use mcp_core::{
    handler::ToolError,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
    role::Role,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

/// An extension for controlling Android devices through ADB
#[derive(Clone)]
pub struct GoslingRouter {
    tools: Vec<Tool>,
    active_resources: Arc<Mutex<HashMap<String, Resource>>>,
    instructions: String,
}

impl Default for GoslingRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl GoslingRouter {
    pub fn new() -> Self {
        // Create tools for the system
        let screenshot_tool = Tool::new(
            "screenshot",
            "Take a screenshot of the current Android device or emulator",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        );

        let home_tool = Tool::new(
            "home",
            "Press the home button on the device",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        );

        let click_tool = Tool::new(
            "click",
            "Click at specific coordinates on the device screen",
            json!({
                "type": "object",
                "required": ["x", "y"],
                "properties": {
                    "x": {
                        "type": "integer",
                        "description": "X coordinate to click"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate to click"
                    }
                }
            }),
        );

        let enter_text_tool = Tool::new(
            "enter_text",
            indoc! {r#"
                Enter text into the current text field. Text will automatically submit unless you end on ---
            "#},
            json!({
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to enter"
                    }
                }
            }),
        );

        let start_app_tool = Tool::new(
            "start_app",
            "Start an application by its package name",
            json!({
                "type": "object",
                "required": ["package_name"],
                "properties": {
                    "package_name": {
                        "type": "string",
                        "description": "Full package name of the app to start"
                    }
                }
            }),
        );

        let select_text_tool = Tool::new(
            "select_text",
            "Select text on the screen between two coordinate points",
            json!({
                "type": "object",
                "required": ["start_x", "start_y", "end_x", "end_y"],
                "properties": {
                    "start_x": {
                        "type": "integer",
                        "description": "Starting X coordinate for text selection"
                    },
                    "start_y": {
                        "type": "integer",
                        "description": "Starting Y coordinate for text selection"
                    },
                    "end_x": {
                        "type": "integer",
                        "description": "Ending X coordinate for text selection"
                    },
                    "end_y": {
                        "type": "integer",
                        "description": "Ending Y coordinate for text selection"
                    }
                }
            }),
        );

        let swipe_tool = Tool::new(
            "swipe",
            "Swipe from one point to another on the screen for example to scroll",
            json!({
                "type": "object",
                "required": ["start_x", "start_y", "end_x", "end_y"],
                "properties": {
                    "start_x": {
                        "type": "integer",
                        "description": "Starting X coordinate"
                    },
                    "start_y": {
                        "type": "integer",
                        "description": "Starting Y coordinate"
                    },
                    "end_x": {
                        "type": "integer",
                        "description": "Ending X coordinate"
                    },
                    "end_y": {
                        "type": "integer",
                        "description": "Ending Y coordinate"
                    },
                    "duration": {
                        "type": "integer",
                        "description": "Duration of swipe in milliseconds. Default is 300. Use longer duration (500+) for text selection",
                        "default": 300
                    }
                }
            }),
        );

        let copy_selected_tool = Tool::new(
            "copy_selected",
            "Copy currently selected text to clipboard and return the value",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        );

        let instructions = indoc! {r#"
            This extension provides tools for controlling an Android device through ADB.
            
            After each interactive command you will see:
            1. A screenshot of the current device state
            2. The UI hierarchy information
            Use these to verify your actions.
            
            Available Tools:
            - Press home button
            - Click at coordinates
            - Enter text (use --- at end to skip auto-submit)
            - Start apps by package name
            - Select and copy text
            - Swipe for scrolling
            - Take screenshots
            "#};

        Self {
            tools: vec![
                screenshot_tool,
                home_tool,
                click_tool,
                enter_text_tool,
                start_app_tool,
                select_text_tool,
                swipe_tool,
                copy_selected_tool,
            ],
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            instructions: instructions.to_string(),
        }
    }

    async fn run_adb(&self, args: &[&str]) -> Result<Vec<u8>, ToolError> {
        let output = Command::new("adb")
            .args(args)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to run adb command: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionError(format!(
                "ADB command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(output.stdout)
    }

    async fn get_ui_hierarchy(&self) -> Result<String, ToolError> {
        self.run_adb(&["shell", "uiautomator", "dump", "/sdcard/window_dump.xml"]).await?;
        let xml_data = self.run_adb(&["shell", "cat", "/sdcard/window_dump.xml"]).await?;
        Ok(String::from_utf8_lossy(&xml_data).to_string())
    }

    async fn process_screenshot(&self, data: Vec<u8>) -> Result<String, ToolError> {
        // Load image from bytes
        let img = image::load_from_memory(&data)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to load image: {}", e)))?;

        // Resize if width > 768px
        let img = if img.width() > 768 {
            let ratio = 768.0 / img.width() as f32;
            let new_height = (img.height() as f32 * ratio) as u32;
            img.resize(768, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        // Convert to RGB and save as JPEG
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to encode image: {}", e)))?;

        Ok(BASE64.encode(&buffer))
    }

    async fn take_screenshot(&self) -> Result<Vec<Content>, ToolError> {
        let screenshot_data = self.run_adb(&["shell", "screencap", "-p"]).await?;
        let processed_data = self.process_screenshot(screenshot_data).await?;
        
        Ok(vec![
            Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
            Content::image(processed_data, "image/jpeg")
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    async fn home(&self) -> Result<Vec<Content>, ToolError> {
        self.run_adb(&["shell", "input", "keyevent", "KEYCODE_HOME"]).await?;
        Ok(vec![Content::text("Pressed home button")])
    }

    async fn click(&self, x: i32, y: i32) -> Result<Vec<Content>, ToolError> {
        self.run_adb(&["shell", "input", "tap", &x.to_string(), &y.to_string()]).await?;
        Ok(vec![Content::text(format!("Clicked at coordinates ({}, {})", x, y))])
    }

    async fn enter_text(&self, text: &str) -> Result<Vec<Content>, ToolError> {
        for line in text.split('\n') {
            let mut line = line.to_string();
            let skip_auto_submit = line.ends_with("---");
            if skip_auto_submit {
                line = line[..line.len() - 3].to_string();
            }
            
            // Replace problematic characters
            let line = line.replace('\'', " ")
                         .replace('€', "EUR")
                         .replace('ö', "o");

            self.run_adb(&["shell", "input", "text", &format!("'{}'", line)]).await?;
            
            if !skip_auto_submit {
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                self.run_adb(&["shell", "input", "keyevent", "KEYCODE_ENTER"]).await?;
            }
        }
        Ok(vec![Content::text(format!("Entered text: '{}'", text))])
    }

    async fn start_app(&self, package_name: &str) -> Result<Vec<Content>, ToolError> {
        self.run_adb(&[
            "shell",
            "monkey",
            "-p",
            package_name,
            "-c",
            "android.intent.category.LAUNCHER",
            "1",
        ])
        .await?;
        Ok(vec![Content::text(format!("Started app: {}", package_name))])
    }

    async fn select_text(
        &self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
    ) -> Result<Vec<Content>, ToolError> {
        // Long press at starting point
        self.run_adb(&[
            "shell",
            "input",
            "swipe",
            &start_x.to_string(),
            &start_y.to_string(),
            &start_x.to_string(),
            &start_y.to_string(),
            "800",
        ])
        .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Drag to select
        self.run_adb(&[
            "shell",
            "input",
            "swipe",
            &start_x.to_string(),
            &start_y.to_string(),
            &end_x.to_string(),
            &end_y.to_string(),
            "300",
        ])
        .await?;

        Ok(vec![Content::text(format!(
            "Selected text from ({}, {}) to ({}, {})",
            start_x, start_y, end_x, end_y
        ))])
    }

    async fn swipe(
        &self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        duration: i32,
    ) -> Result<Vec<Content>, ToolError> {
        self.run_adb(&[
            "shell",
            "input",
            "swipe",
            &start_x.to_string(),
            &start_y.to_string(),
            &end_x.to_string(),
            &end_y.to_string(),
            &duration.to_string(),
        ])
        .await?;

        Ok(vec![Content::text(format!(
            "Swiped from ({}, {}) to ({}, {})",
            start_x, start_y, end_x, end_y
        ))])
    }

    async fn copy_selected(&self) -> Result<Vec<Content>, ToolError> {
        self.run_adb(&["shell", "input", "keyevent", "KEYCODE_COPY"]).await?;
        let clipboard_data = self.run_adb(&["shell", "service", "call", "clipboard", "get_clipboard"]).await?;
        Ok(vec![Content::text(format!("Clipboard contents: {}", String::from_utf8_lossy(&clipboard_data)))])
    }

    async fn update_device_state(&self) -> Result<Vec<Content>, ToolError> {
        let mut results = Vec::new();

        // Add screenshot
        if let Ok(screenshot) = self.take_screenshot().await {
            results.extend(screenshot);
        }

        // Add UI hierarchy
        if let Ok(hierarchy) = self.get_ui_hierarchy().await {
            results.push(Content::text(format!("UI Hierarchy:\n{}", hierarchy)));
        }

        Ok(results)
    }
}

impl Router for GoslingRouter {
    fn name(&self) -> String {
        "GoslingExtension".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            let mut result = match tool_name.as_str() {
                "screenshot" => this.take_screenshot().await,
                "home" => this.home().await,
                "click" => {
                    let x = arguments.get("x").and_then(|v| v.as_i64()).ok_or_else(|| {
                        ToolError::InvalidParameters("Missing or invalid 'x' parameter".into())
                    })? as i32;
                    let y = arguments.get("y").and_then(|v| v.as_i64()).ok_or_else(|| {
                        ToolError::InvalidParameters("Missing or invalid 'y' parameter".into())
                    })? as i32;
                    this.click(x, y).await
                }
                "enter_text" => {
                    let text = arguments.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'text' parameter".into())
                    })?;
                    this.enter_text(text).await
                }
                "start_app" => {
                    let package_name = arguments
                        .get("package_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing 'package_name' parameter".into())
                        })?;
                    this.start_app(package_name).await
                }
                "select_text" => {
                    let start_x = arguments
                        .get("start_x")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'start_x' parameter".into())
                        })? as i32;
                    let start_y = arguments
                        .get("start_y")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'start_y' parameter".into())
                        })? as i32;
                    let end_x = arguments
                        .get("end_x")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'end_x' parameter".into())
                        })? as i32;
                    let end_y = arguments
                        .get("end_y")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'end_y' parameter".into())
                        })? as i32;
                    this.select_text(start_x, start_y, end_x, end_y).await
                }
                "swipe" => {
                    let start_x = arguments
                        .get("start_x")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'start_x' parameter".into())
                        })? as i32;
                    let start_y = arguments
                        .get("start_y")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'start_y' parameter".into())
                        })? as i32;
                    let end_x = arguments
                        .get("end_x")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'end_x' parameter".into())
                        })? as i32;
                    let end_y = arguments
                        .get("end_y")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing or invalid 'end_y' parameter".into())
                        })? as i32;
                    let duration = arguments
                        .get("duration")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(300) as i32;
                    this.swipe(start_x, start_y, end_x, end_y, duration).await
                }
                "copy_selected" => this.copy_selected().await,
                _ => return Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }?;

            // Add device state update after interactive commands
            if tool_name != "screenshot" {
                if let Ok(state_update) = this.update_device_state().await {
                    result.extend(state_update);
                }
            }

            Ok(result)
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        let active_resources = self.active_resources.lock().unwrap();
        active_resources.values().cloned().collect()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, mcp_core::handler::ResourceError>> + Send + 'static>>
    {
        Box::pin(async move {
            Err(mcp_core::handler::ResourceError::NotFound(
                "Resource not found".into(),
            ))
        })
    }
}