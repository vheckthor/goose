use anyhow::Result;
use serde_json::{json, Value};
use std::{future::Future, path::PathBuf, pin::Pin, process::Command};

use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

pub struct EditorModeRouter {
    tools: Vec<Tool>,
    working_dir: PathBuf,
}

impl Default for EditorModeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorModeRouter {
    pub fn new() -> Self {
        let git_status_tool = Tool::new(
            "status".to_string(),
            "Get the current git repository status.".to_string(),
            json!({
                "type": "object",
                "required": [],
                "properties": {}
            }),
        );

        let git_init_branch_tool = Tool::new(
            "init_branch".to_string(),
            "Initialize a new branch for changes. Will fail if repo is not clean.".to_string(),
            json!({
                "type": "object",
                "required": ["branch_name"],
                "properties": {
                    "branch_name": {"type": "string", "description": "Name of the branch to create"}
                }
            }),
        );

        let git_show_diff_tool = Tool::new(
            "show_diff".to_string(),
            "Show the current changes as a diff.".to_string(),
            json!({
                "type": "object",
                "required": [],
                "properties": {}
            }),
        );

        let git_commit_tool = Tool::new(
            "commit".to_string(),
            "Commit the current changes.".to_string(),
            json!({
                "type": "object",
                "required": ["message"],
                "properties": {
                    "message": {"type": "string", "description": "Commit message"}
                }
            }),
        );

        Self {
            tools: vec![
                git_status_tool,
                git_init_branch_tool,
                git_show_diff_tool,
                git_commit_tool,
            ],
            working_dir: std::env::current_dir().expect("Failed to get current directory"),
        }
    }

    fn is_git_repo(&self) -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(&self.working_dir)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn is_repo_clean(&self) -> bool {
        Command::new("git")
            .args(["diff", "--quiet", "HEAD"])
            .current_dir(&self.working_dir)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    async fn git_status(&self) -> Result<Vec<Content>, ToolError> {
        if !self.is_git_repo() {
            return Err(ToolError::ExecutionError(
                "Not in a git repository".to_string(),
            ));
        }

        let output = Command::new("git")
            .args(["status", "--porcelain=v2"])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let status = String::from_utf8_lossy(&output.stdout);
        Ok(vec![Content::text(status.to_string())])
    }

    async fn git_init_branch(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        if !self.is_git_repo() {
            return Err(ToolError::ExecutionError(
                "Not in a git repository".to_string(),
            ));
        }

        if !self.is_repo_clean() {
            return Err(ToolError::ExecutionError(
                "Repository has uncommitted changes".to_string(),
            ));
        }

        let branch_name = params
            .get("branch_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParameters("branch_name parameter is required".to_string())
            })?;

        // Create and checkout new branch
        let output = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if !output.status.success() {
            return Err(ToolError::ExecutionError(format!(
                "Failed to create branch: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(vec![Content::text(format!(
            "Created and switched to new branch '{}'",
            branch_name
        ))])
    }

    async fn git_show_diff(&self) -> Result<Vec<Content>, ToolError> {
        if !self.is_git_repo() {
            return Err(ToolError::ExecutionError(
                "Not in a git repository".to_string(),
            ));
        }

        let output = Command::new("git")
            .args(["diff"])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let diff = String::from_utf8_lossy(&output.stdout);

        // Format the diff as markdown code block
        let formatted_diff = format!("```diff\n{}\n```", diff);

        Ok(vec![Content::text(formatted_diff)])
    }

    async fn git_commit(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        if !self.is_git_repo() {
            return Err(ToolError::ExecutionError(
                "Not in a git repository".to_string(),
            ));
        }

        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParameters("message parameter is required".to_string())
            })?;

        // Stage all changes
        let stage_output = Command::new("git")
            .args(["add", "."])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if !stage_output.status.success() {
            return Err(ToolError::ExecutionError(
                "Failed to stage changes".to_string(),
            ));
        }

        // Commit changes
        let commit_output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if !commit_output.status.success() {
            return Err(ToolError::ExecutionError(format!(
                "Failed to commit: {}",
                String::from_utf8_lossy(&commit_output.stderr)
            )));
        }

        Ok(vec![Content::text(format!(
            "Changes committed with message: {}",
            message
        ))])
    }
}

impl Router for EditorModeRouter {
    fn name(&self) -> String {
        "editor_mode".to_string()
    }

    fn instructions(&self) -> String {
        let is_git = self.is_git_repo();
        let is_clean = is_git && self.is_repo_clean();

        format!(
            r#"The editor mode extension provides tools for making changes to code with git integration.
This ensures all changes are tracked and can be reviewed before being committed.

Current Status:
- In git repository: {}
- Repository is clean: {}

This extension requires:
1. Git to be installed and configured
2. A clean git repository (no uncommitted changes when starting)

Workflow:
1. Use status to check repository state
2. Create a new branch with init_branch
3. Make changes using other tools (e.g., developer extension)
4. Review changes with show_diff
5. Commit changes with commit

Use this extension alongside the developer extension for a complete editing workflow.
Once your changes are ready, use git directly or other tools to push and create PRs.
"#,
            if is_git { "Yes" } else { "No" },
            if is_clean { "Yes" } else { "No" },
        )
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_prompts(false)
            .build()
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
        // Format error string before the async block to avoid capturing the reference
        let not_found = format!("Tool {} not found", tool_name);
        match tool_name {
            "status" => Box::pin(async move { this.git_status().await }),
            "init_branch" => Box::pin(async move { this.git_init_branch(arguments).await }),
            "show_diff" => Box::pin(async move { this.git_show_diff().await }),
            "commit" => Box::pin(async move { this.git_commit(arguments).await }),
            _ => Box::pin(async move { Err(ToolError::NotFound(not_found)) }),
        }
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
    }

    fn list_prompts(&self) -> Vec<mcp_core::prompt::Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
    }
}

impl Clone for EditorModeRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            working_dir: self.working_dir.clone(),
        }
    }
}
