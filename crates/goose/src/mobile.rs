use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::Cursor;
use anyhow::Result as AnyhowResult;
use crate::errors::{AgentError, AgentResult};
use async_trait::async_trait;
use base64::prelude::*;
use indoc::{formatdoc, indoc};
use serde_json::{json, Value};
use adb_client::{ADBServer, ADBDeviceExt};
use scraper::{Html, Selector};
use reqwest::Client;
use crate::models::tool::{Tool, ToolCall};
use crate::models::content::Content;
use crate::systems::System;
use std::sync::Arc;
use tokio::sync::RwLock;
use image::{ImageFormat, GenericImageView, imageops::FilterType};



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
                Interact with an Android device or emulator. You control the device on behalf of the user; the
                user cannot see the device or interact with it directly. You can only interact with the device.


            "#},
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "enum": ["home", "click", "enter_text", "screenshot", "list_apps", "start_app"],
                        "description": "The commands to run."
                    },
                    "location": {
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
                                "description": "Text to enter. Add a newline to submit."
                            }
                        },
                        "required": ["text"]
                    },
                    "start_app": {
                        "type": "object",
                        "properties": {
                            "package_name": {
                                "type": "string",
                                "description": "Full package name of the app to start."
                            }
                        },
                        "required": ["package_name"]
                    }
                },
                "required": ["command"]
            }),
        );
        let instructions = formatdoc! {r#"
            Fullfill the requests of the user by sending commands to the Android device. Break up each requests into steps
            and accomplish those steps by starting the right app on the device and interacting with it. Verify anything you
            type of click; don't assume it worked. You can find the UI hierarchy in the status to see what the screen
            looks like. Make sure what you did worked by checking the UI hierarchy before moving on to the next step.

            For example, when asked to send the current weather by email to user john, start chrome and go to weather.com
            and verify that it shows the weather. Then start gmail and navigate to compose. Then enter "john" in the "To"
            field and look for the auto-complete. Enter "Current weather" in the subject and the actual weather in the
            body. Then check that it all looks good before hitting the send button.
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

    async fn get_ui_hierarchy(&self) -> Result<String, AgentError> {
        self.run_shell_command(&["uiautomator", "dump", "/sdcard/window_dump.xml"]).await?;
        
        let xml_content = self.run_shell_command(&["cat", "/sdcard/window_dump.xml"]).await?;
        let ui_dump = String::from_utf8(xml_content)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to parse XML content: {}", e)))?;

        // Write to file for debugging/reference
        std::fs::write("uihierarchy.xml", &ui_dump).map_err(|e| {
            AgentError::ExecutionError(format!("Failed to write UI hierarchy: {}", e))
        })?;

        Ok(ui_dump)
    }

    async fn get_app_description(&self, package_name: &str) -> Option<String> {
        let cache_path = Path::new("app_cache.json");
    
        let mut cache: HashMap<String, Option<String>> = if cache_path.exists() {
            let data = fs::read_to_string(cache_path).ok()?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
    
        if let Some(description) = cache.get(package_name) {
            return description.clone();
        }
    
        let title = self.scrape_play_store_title(package_name).await;
    
        cache.insert(package_name.to_string(), title.clone());
    
        let serialized_cache = serde_json::to_string(&cache).ok()?;
        fs::write(cache_path, serialized_cache).ok()?;
    
        title
    }

    async fn scrape_play_store_title(&self, package_name: &str) -> Option<String> {
        let client = Client::new();
        let url = format!("https://play.google.com/store/apps/details?id={}", package_name);
    
        let response = client.get(&url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
    
        let body = response.text().await.ok()?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("h1 span.AfwdI[itemprop='name']").ok()?;
    
        if let Some(span) = document.select(&selector).next() {
            if let Some(title) = span.text().next() {
                return Some(title.to_string());
            }
        }
    
        None
    }

    async fn list_installed_apps(&self) -> Result<Vec<(String, String)>, AgentError> {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| AgentError::ExecutionError("Device not connected.".to_string()))?;

        let mut device = device.write().await;
        let mut output = Vec::new();

        device
            .shell_command(&["pm", "list", "packages"], &mut output)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to list packages: {}", e)))?;

        let packages = String::from_utf8_lossy(&output)
            .lines()
            .filter_map(|line| {
                line.strip_prefix("package:").map(|pkg| pkg.trim().to_string())
            })
            .collect::<Vec<String>>();

        let mut apps_with_descriptions = Vec::new();
        for package in packages {
            if let Some(description) = self.get_app_description(&package).await {
                apps_with_descriptions.push((package, description));
            }
        }
    
        Ok(apps_with_descriptions)
    }

    fn process_screenshot(screenshot_data: &[u8]) -> Result<Vec<u8>, AgentError> {
        let img = image::load_from_memory(screenshot_data)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to load image: {}", e)))?;
        
        const MAX_WIDTH: u32 = 768;
        let (width, height) = img.dimensions();
        
        // Resize if needed
        let processed = if width > MAX_WIDTH {
            let ratio = MAX_WIDTH as f32 / width as f32;
            let new_height = (height as f32 * ratio) as u32;
            img.resize(MAX_WIDTH, new_height, FilterType::Lanczos3)
        } else {
            img
        };
        
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            processed.write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to encode image as JPEG: {}", e)))?;
        }
        
        Ok(buffer)
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

        if self.device.is_some() {
            if let Ok(ui_hierarchy) = self.get_ui_hierarchy().await {
                status.insert("ui_hierarchy".to_string(), json!(ui_hierarchy));
            }
        }

        Ok(status)
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        println!("Received tool call: {}", serde_json::to_string_pretty(&tool_call).unwrap_or_default());


        match tool_call.name.as_str() {
            "android" => {
                let response_message = match self.get_argument(&tool_call, "command", None).and_then(Value::as_str) {
                    Some("home") => {
                        self.run_shell_command(&["input", "keyevent", "KEYCODE_HOME"]).await?;
                        Content::text("Sent home key event.".to_string())
                    }
                    Some("click") => {
                        let x = self.get_argument(&tool_call, "location", Some("x")).and_then(Value::as_i64);
                        let y = self.get_argument(&tool_call, "location", Some("y")).and_then(Value::as_i64);
    
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
                            let lines: Vec<&str> = text.split('\n').collect();
                            for line in lines {
                                let command = format!("input text '{}'", line);
                                self.run_shell_command(&command.split_whitespace().collect::<Vec<&str>>()).await?;
                                self.run_shell_command(&["input", "keyevent", "KEYCODE_ENTER"]).await?;
                            }
                            self.run_shell_command(&["input", "keyevent", "KEYCODE_ENTER"]).await?;
                            Content::text(format!("Entered text: '{}'.", text))
                        } else {
                            return Err(AgentError::ExecutionError("Missing or invalid text input.".to_string()));
                        }
                    }
                    Some("screenshot") => {
                        let screenshot_data = self.run_shell_command(&["screencap", "-p"]).await?;
                        let processed_data = Self::process_screenshot(&screenshot_data)?;
                        
                        std::fs::write("screenshot.jpg", &processed_data).map_err(|e| {
                            AgentError::ExecutionError(format!("Failed to write screenshot: {}", e))
                        })?;
    
                        let image_data = BASE64_STANDARD.encode(&processed_data);
                        Content::image(image_data, "image/jpeg")
                    }
                    Some("list_apps") => {
                        let apps = self.list_installed_apps().await?;
                        let formatted_apps = apps
                            .into_iter()
                            .map(|(package, description)| format!("{}: {}", package, description))
                            .collect::<Vec<String>>()
                            .join("\n");
                    
                        Content::text(format!("Installed Apps:\n{}", formatted_apps))
                    }
                    Some("start_app") => {
                        let package_name = self.get_argument(&tool_call, "start_app", Some("package_name")).and_then(Value::as_str);
    
                        if let Some(package) = package_name {
                            let command = format!("monkey -p {} -c android.intent.category.LAUNCHER 1", package);
                            self.run_shell_command(&command.split_whitespace().collect::<Vec<&str>>()).await?;
                            Content::text(format!("Started app: {}", package))
                        } else {
                            return Err(AgentError::ExecutionError("Missing or invalid package name.".to_string()));
                        }
                    }
                    Some(cmd) => return Err(AgentError::ExecutionError(format!("Invalid or unsupported command: '{}'", cmd))),
                    _ => return Err(AgentError::ExecutionError("Invalid or unsupported command.".to_string())),
                };
    
                Ok(vec![response_message])

            }
            _ => Err(AgentError::ExecutionError("Unknown tool name.".to_string())),
        }
    }    
    
}
