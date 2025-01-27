use indoc::indoc;
use serde_json::json;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, sync::Mutex};
use tokio::process::Command;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use mcp_core::{
    handler::ToolError,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
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
            You can perform actions like:
            - Pressing the home button
            - Clicking at specific coordinates
            - Entering text
            - Starting apps
            - Selecting and copying text
            - Swiping/scrolling

            The extension automatically handles:
            - ADB command execution
            - Text input processing
            - Screenshot capture and processing
            - UI hierarchy inspection
            "#};

        Self {
            tools: vec![
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

    async fn run_adb_shell(&self, command: Vec<&str>) -> Result<String, ToolError> {
        let mut adb_command = vec!["shell"];
        adb_command.extend(command);

        let output = Command::new("adb")
            .args(&adb_command)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to run adb command: {}", e)))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionError(format!(
                "ADB command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn take_screenshot(&self) -> Result<String, ToolError> {
        let screenshot_data = self.run_adb_shell(vec!["screencap", "-p"]).await?;
        // In a real implementation, we would process the image here like in the Python version
        // For now, we'll just base64 encode it
        Ok(BASE64.encode(screenshot_data.as_bytes()))
    }

    async fn get_ui_hierarchy(&self) -> Result<String, ToolError> {
        self.run_adb_shell(vec!["uiautomator", "dump", "/sdcard/window_dump.xml"]).await?;
        self.run_adb_shell(vec!["cat", "/sdcard/window_dump.xml"]).await
    }

    async fn home(&self) -> Result<Vec<Content>, ToolError> {
        self.run_adb_shell(vec!["input", "keyevent", "KEYCODE_HOME"]).await?;
        Ok(vec![Content::text("Pressed home button")])
    }

    async fn click(&self, x: i32, y: i32) -> Result<Vec<Content>, ToolError> {
        self.run_adb_shell(vec!["input", "tap", &x.to_string(), &y.to_string()]).await?;
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

            self.run_adb_shell(vec!["input", "text", &format!("'{}'", line)]).await?;
            
            if !skip_auto_submit {
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                self.run_adb_shell(vec!["input", "keyevent", "KEYCODE_ENTER"]).await?;
            }
        }
        Ok(vec![Content::text(format!("Entered text: '{}'", text))])
    }

    async fn start_app(&self, package_name: &str) -> Result<Vec<Content>, ToolError> {
        self.run_adb_shell(vec![
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
        self.run_adb_shell(vec![
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
        self.run_adb_shell(vec![
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
        self.run_adb_shell(vec![
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
        self.run_adb_shell(vec!["input", "keyevent", "KEYCODE_COPY"]).await?;
        let clipboard_content = self.run_adb_shell(vec!["service", "call", "clipboard", "get_clipboard"]).await?;
        Ok(vec![Content::text(format!("Clipboard contents: {}", clipboard_content))])
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
            match tool_name.as_str() {
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
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
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