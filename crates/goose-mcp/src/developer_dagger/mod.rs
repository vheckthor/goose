use anyhow::Result;
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    process::{Command as StdCommand, Stdio as StdStdio},
};
use tokio::process::Command;

use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_core::prompt::Prompt;
use mcp_server::Router;
use mcp_core::role::Role;

use indoc::indoc;

/// DeveloperDaggerRouter is a router that handles shell commands through Dagger.
/// It delegates text editor and other operations to the original DeveloperRouter.
pub struct DeveloperDaggerRouter {
    developer_router: crate::developer::DeveloperRouter,
    tools: Vec<Tool>,
}

impl Default for DeveloperDaggerRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperDaggerRouter {
    pub fn new() -> Self {
        // Get the original tools from DeveloperRouter
        let developer_router = crate::developer::DeveloperRouter::new();
        let original_tools = developer_router.list_tools();
        
        // Create our modified tools for bash only
        let mut tools = Vec::new();
        
        for tool in original_tools {
            if tool.name == "shell" {
                // Create a modified shell tool for Dagger
                let dagger_shell_tool = Tool::new(
                    "shell".to_string(),
                    indoc! {r#"
                        Execute a command in the shell within a Dagger container.

                        This will return the output and error concatenated into a single string, as
                        you would see from running on the command line.

                        The command will be executed within a Dagger container using the specified image.
                        You can choose between copying files into the container (copy-on-write) or mounting them.

                        Avoid commands that produce a large amount of output, and consider piping those outputs to files.
                    "#}.to_string(),
                    json!({
                        "type": "object",
                        "required": ["command"],
                        "properties": {
                            "command": {"type": "string"},
                            "container_image": {
                                "type": "string",
                                "default": "alpine:latest",
                                "description": "The container image to use"
                            },
                            "mount_mode": {
                                "type": "string",
                                "enum": ["copy", "mount"],
                                "default": "copy",
                                "description": "Whether to copy files into the container (copy) or mount them (mount)"
                            }
                        }
                    }),
                    None,
                );
                tools.push(dagger_shell_tool);
            } else {
                // Keep all other tools as is
                tools.push(tool);
            }
        }

        Self {
            developer_router,
            tools,
        }
    }

    // Helper method to check if Dagger is installed
    async fn check_dagger_installed(&self) -> Result<bool, ToolError> {
        let output = StdCommand::new("dagger")
            .arg("version")
            .stdout(StdStdio::null())
            .stderr(StdStdio::null())
            .status();

        match output {
            Ok(_) => Ok(true),
            Err(_) => Err(ToolError::ExecutionError(
                "Dagger CLI is not installed. Please install it from https://docs.dagger.io/".to_string(),
            )),
        }
    }

    // Shell command execution with Dagger
    async fn bash_dagger(
        &self, 
        command: &str, 
        container_image: &str,
        mount_mode: &str
    ) -> Result<String, ToolError> {
        // Check if Dagger is installed
        self.check_dagger_installed().await?;

        // Get current directory
        let cwd = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get current directory: {}", e)))?;

        // Prepare the Dagger command
        let mount_type = match mount_mode {
            "mount" => "with-mounted-directory",
            _ => "with-directory"
        };

        // Build the Dagger command
        let dagger_command = format!(
            "dagger core container from --address=\"{}\" {} --path=\"/workspace\" --directory=\".\" with-workdir --path=\"/workspace\" with-exec --args=\"sh\",\"-c\",\"{}\" stdout",
            container_image,
            mount_type,
            command.replace("\"", "\\\"")
        );

        // Execute the Dagger command using the shell
        let output = Command::new("sh")
            .arg("-c")
            .arg(&dagger_command)
            .current_dir(&cwd)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to execute Dagger command: {}", e)))?;

        // Get the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Check if the command was successful
        if !output.status.success() {
            return Err(ToolError::ExecutionError(format!(
                "Dagger command failed: {}\n{}",
                dagger_command,
                stderr
            )));
        }

        // Return the combined output
        Ok(format!("{}\n{}", stdout, stderr))
    }

    async fn bash(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let command =
            params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The command string is required".to_string(),
                ))?;

        // Get container image and mount mode
        let container_image = params
            .get("container_image")
            .and_then(|v| v.as_str())
            .unwrap_or("alpine:latest");

        let mount_mode = params
            .get("mount_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("copy");

        // Execute the command using Dagger
        let output = self.bash_dagger(command, container_image, mount_mode).await?;

        // Check the character count of the output
        const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB
        let char_count = output.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(ToolError::ExecutionError(format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command,
                    char_count,
                    MAX_CHAR_COUNT
                )));
        }

        // Format the output to indicate it was run in a Dagger container
        let formatted_output = format!(
            "# Command executed in Dagger container: {}\n# Mount mode: {}\n\n{}",
            container_image,
            mount_mode,
            output
        );

        Ok(vec![
            Content::text(formatted_output.clone()).with_audience(vec![Role::Assistant]),
            Content::text(formatted_output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }
}

impl Router for DeveloperDaggerRouter {
    fn name(&self) -> String {
        "developer-dagger".to_string()
    }

    fn instructions(&self) -> String {
        let mut instructions = self.developer_router.instructions();
        
        // Add Dagger-specific instructions
        instructions.push_str("\n\n### Dagger Integration\n");
        instructions.push_str("This extension runs shell commands within Dagger containers while keeping file operations unchanged.\n");
        instructions.push_str("You can specify the container image with `container_image` and choose between copy-on-write or mounting with `mount_mode`.\n");
        instructions.push_str("\nExample: `shell(command: \"ls -la\", container_image: \"node:18\", mount_mode: \"copy\")`\n");
        
        instructions
    }

    fn capabilities(&self) -> ServerCapabilities {
        self.developer_router.capabilities()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "shell" => this.bash(arguments).await,
                // For all other tools, delegate to the original DeveloperRouter
                _ => this.developer_router.call_tool(&tool_name, arguments).await,
            }
        })
    }

    // Delegate other methods to the original DeveloperRouter
    fn list_resources(&self) -> Vec<Resource> {
        self.developer_router.list_resources()
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        self.developer_router.read_resource(uri)
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        self.developer_router.list_prompts()
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        self.developer_router.get_prompt(prompt_name)
    }
}

impl Clone for DeveloperDaggerRouter {
    fn clone(&self) -> Self {
        Self {
            developer_router: self.developer_router.clone(),
            tools: self.tools.clone(),
        }
    }
}