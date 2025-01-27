use indoc::indoc;
use serde_json::json;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, sync::Mutex};
use tokio::process::Command;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::path::PathBuf;
use std::env;

use mcp_core::{
    handler::ToolError,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

const DEFAULT_EMU_NAME: &str = "goose_pixel_7";
const DEFAULT_EMU_DEVICE: &str = "pixel_7"; // or "pixel_7_pro", etc

/// An extension for controlling Android devices through ADB
#[derive(Clone)]
pub struct GoslingRouter {
    tools: Vec<Tool>,
    active_resources: Arc<Mutex<HashMap<String, Resource>>>,
    instructions: String,
    sdk_path: Option<PathBuf>,
}

impl Default for GoslingRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl GoslingRouter {
    pub fn new() -> Self {
        // Create tools for the system
        let check_environment_tool = Tool::new(
            "check_environment",
            "Check if Android SDK and emulator are properly set up",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        );

        let setup_environment_tool = Tool::new(
            "setup_environment",
            "Install Android SDK and set up emulator if needed",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        );

        let start_emulator_tool = Tool::new(
            "start_emulator",
            "Start the Android emulator",
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the emulator to start (optional)"
                    }
                },
                "required": []
            }),
        );

        let list_emulators_tool = Tool::new(
            "list_emulators",
            "List available Android emulators",
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
            This extension provides tools for controlling an Android device or emulator through ADB.
            
            Environment Management:
            - Check if Android SDK and emulator are installed
            - Set up Android development environment if needed
            - List and manage Android emulators
            
            Device Control:
            - Press home button
            - Click at coordinates
            - Enter text
            - Start apps
            - Select and copy text
            - Swipe/scroll
            
            The extension automatically handles:
            - Android SDK detection and setup
            - Emulator management
            - ADB command execution
            - Text input processing
            - Screenshot capture and processing
            - UI hierarchy inspection
            "#};

        let mut router = Self {
            tools: vec![
                check_environment_tool,
                setup_environment_tool,
                start_emulator_tool,
                list_emulators_tool,
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
            sdk_path: None,
        };

        // Try to locate Android SDK
        if let Ok(path) = router.find_android_sdk() {
            router.sdk_path = Some(path);
        }

        router
    }

    fn find_android_sdk(&self) -> Result<PathBuf, ToolError> {
        // Check common locations for Android SDK
        let home = env::var("HOME").map_err(|_| ToolError::ExecutionError("Could not determine home directory".into()))?;
        
        let possible_paths = vec![
            // Android Studio default location
            format!("{}/Library/Android/sdk", home),
            // Homebrew location
            "/usr/local/share/android-sdk".to_string(),
            // Command line tools location
            format!("{}/Library/Android/cmdline-tools", home),
        ];

        for path in possible_paths {
            let path = PathBuf::from(&path);
            if path.exists() && path.join("platform-tools").exists() {
                return Ok(path);
            }
        }

        Err(ToolError::ExecutionError("Android SDK not found".into()))
    }

    async fn check_environment(&self) -> Result<Vec<Content>, ToolError> {
        let mut status = vec![];

        // Check if we found SDK path
        match &self.sdk_path {
            Some(path) => {
                status.push(format!("✓ Android SDK found at: {}", path.display()));
                
                // Check for essential components
                let components = [
                    ("platform-tools/adb", "ADB"),
                    ("emulator/emulator", "Emulator"),
                    ("cmdline-tools/latest/bin/sdkmanager", "SDK Manager"),
                    ("cmdline-tools/latest/bin/avdmanager", "AVD Manager"),
                ];

                for (path_suffix, name) in components {
                    if path.join(path_suffix).exists() {
                        status.push(format!("✓ {} is installed", name));
                    } else {
                        status.push(format!("✗ {} is not installed", name));
                    }
                }
            }
            None => {
                status.push("✗ Android SDK not found".to_string());
            }
        };

        // Check for running emulators
        match Command::new("adb").args(["devices"]).output().await {
            Ok(output) => {
                let devices = String::from_utf8_lossy(&output.stdout);
                if devices.lines().count() > 1 {
                    status.push("✓ ADB server is running".to_string());
                    for line in devices.lines().skip(1) {
                        if !line.trim().is_empty() {
                            status.push(format!("  Device: {}", line));
                        }
                    }
                } else {
                    status.push("✗ No devices/emulators connected".to_string());
                }
            }
            Err(_) => {
                status.push("✗ ADB is not available in PATH".to_string());
            }
        }

        Ok(vec![Content::text(status.join("\n"))])
    }

    async fn setup_environment(&self) -> Result<Vec<Content>, ToolError> {
        let mut status = vec![];

        // Check if Homebrew is installed
        if Command::new("brew").arg("--version").output().await.is_err() {
            status.push("Installing Homebrew...");
            let install_script = "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";
            Command::new("bash")
                .arg("-c")
                .arg(install_script)
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to install Homebrew: {}", e)))?;
        }

        // Install Android SDK via Homebrew
        status.push("Installing Android SDK...");
        Command::new("brew")
            .args(["install", "android-sdk"])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to install Android SDK: {}", e)))?;

        // Install command line tools
        status.push("Installing Android command line tools...");
        Command::new("brew")
            .args(["install", "android-commandlinetools"])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to install command line tools: {}", e)))?;

        // Accept licenses
        status.push("Accepting SDK licenses...");
        if let Some(sdk_path) = &self.sdk_path {
            let sdkmanager = sdk_path.join("cmdline-tools/latest/bin/sdkmanager");
            Command::new(&sdkmanager)
                .arg("--licenses")
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to accept licenses: {}", e)))?;

            // Install system image for emulator
            status.push("Installing system image for emulator...");
            Command::new(&sdkmanager)
                .args(["--install", "system-images;android-34;google_apis;arm64-v8a"])
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to install system image: {}", e)))?;

            // Create default emulator
            status.push("Creating default emulator...");
            let avdmanager = sdk_path.join("cmdline-tools/latest/bin/avdmanager");
            Command::new(&avdmanager)
                .args([
                    "create", "avd",
                    "--name", DEFAULT_EMU_NAME,
                    "--device", DEFAULT_EMU_DEVICE,
                    "--package", "system-images;android-34;google_apis;arm64-v8a",
                ])
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to create emulator: {}", e)))?;
        }

        status.push("Environment setup complete!");
        Ok(vec![Content::text(status.join("\n"))])
    }

    async fn list_emulators(&self) -> Result<Vec<Content>, ToolError> {
        if let Some(sdk_path) = &self.sdk_path {
            let emulator = sdk_path.join("emulator/emulator");
            let output = Command::new(&emulator)
                .arg("-list-avds")
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to list emulators: {}", e)))?;

            let emulators = String::from_utf8_lossy(&output.stdout);
            if emulators.trim().is_empty() {
                Ok(vec![Content::text("No emulators found. Use setup_environment to create one.")])
            } else {
                Ok(vec![Content::text(format!("Available emulators:\n{}", emulators))])
            }
        } else {
            Err(ToolError::ExecutionError("Android SDK not found".into()))
        }
    }

    async fn start_emulator(&self, name: Option<String>) -> Result<Vec<Content>, ToolError> {
        let name = name.unwrap_or_else(|| DEFAULT_EMU_NAME.to_string());

        if let Some(sdk_path) = &self.sdk_path {
            let emulator = sdk_path.join("emulator/emulator");
            
            // Check if emulator exists
            let output = Command::new(&emulator)
                .arg("-list-avds")
                .output()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to list emulators: {}", e)))?;

            let emulators = String::from_utf8_lossy(&output.stdout);
            if !emulators.lines().any(|line| line.trim() == name) {
                return Err(ToolError::ExecutionError(format!("Emulator '{}' not found", name)));
            }

            // Start emulator in the background
            let _child = Command::new(&emulator)
                .arg("-avd")
                .arg(&name)
                .arg("-no-window") // Optional: run without window for headless operation
                .spawn()
                .map_err(|e| ToolError::ExecutionError(format!("Failed to start emulator: {}", e)))?;

            // Wait for device to be ready
            let mut attempts = 0;
            while attempts < 30 {
                let output = Command::new("adb")
                    .args(["devices"])
                    .output()
                    .await
                    .map_err(|e| ToolError::ExecutionError(format!("Failed to check devices: {}", e)))?;

                let devices = String::from_utf8_lossy(&output.stdout);
                if devices.lines().count() > 1 {
                    return Ok(vec![Content::text(format!("Emulator '{}' started successfully", name))]);
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                attempts += 1;
            }

            Err(ToolError::ExecutionError("Emulator failed to start within timeout".into()))
        } else {
            Err(ToolError::ExecutionError("Android SDK not found".into()))
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
                "check_environment" => this.check_environment().await,
                "setup_environment" => this.setup_environment().await,
                "list_emulators" => this.list_emulators().await,
                "start_emulator" => {
                    let name = arguments.get("name").and_then(|v| v.as_str()).map(String::from);
                    this.start_emulator(name).await
                },
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