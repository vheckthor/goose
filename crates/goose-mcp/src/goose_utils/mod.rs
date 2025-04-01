use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
    prompt::Prompt,
};
use mcp_server::{router::CapabilitiesBuilder, Router};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin};
use goose::config::{ExtensionConfig, ExtensionManager};
use goose::agents::Capabilities;
use goose::model::ModelConfig;
use goose::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use goose::providers::errors::ProviderError;
use goose::message::Message;
use goose::agents::AgentFactory;

pub struct GooseUtilsRouter {
    tools: Vec<Tool>,
}

impl Default for GooseUtilsRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl GooseUtilsRouter {
    pub fn new() -> Self {
        let discover_extensions = create_discover_extensions_tool();
        let install_extension = create_install_extension_tool();

        Self {
            tools: vec![discover_extensions, install_extension],
        }
    }

    async fn discover_extensions(&self, _params: Value) -> Result<Vec<Content>, ToolError> {
        let mut output_parts = vec![];

        // First get disabled extensions from current config
        let mut disabled_extensions: Vec<String> = vec![];
        for extension in ExtensionManager::get_all().expect("should load extensions") {
            if !extension.enabled {
                let config = extension.config.clone();
                let description = match &config {
                    ExtensionConfig::Builtin {
                        name, display_name, ..
                    } => {
                        // For builtin extensions, use display name if available
                        display_name
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| name.clone())
                    }
                    ExtensionConfig::Sse {
                        description, name, ..
                    }
                    | ExtensionConfig::Stdio {
                        description, name, ..
                    } => {
                        // For SSE/Stdio, use description if available
                        description
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("Extension '{}'", name))
                    }
                };
                disabled_extensions.push(format!("- {} - {}", config.name(), description));
            }
        }

        if !disabled_extensions.is_empty() {
            output_parts.push(format!(
                "Currently installed extensions user can enable:\n{}\n",
                disabled_extensions.join("\n")
            ));
        } else {
            output_parts
                .push("No installed extensions found in current configuration.\n".to_string());
        }

        // Read the servers.json file that's included at compile time
        static SERVERS_JSON: &str = include_str!("../../../../documentation/static/servers.json");

        if let Ok(servers) = serde_json::from_str::<Vec<serde_json::Value>>(SERVERS_JSON) {
            let mut available_extensions: Vec<String> = vec![];
            for server in servers {
                let id = server["id"].as_str().unwrap_or("unknown");
                let name = server["name"].as_str().unwrap_or("Unknown Name");
                let description = server["description"].as_str().unwrap_or("No description");
                let is_builtin = server["is_builtin"].as_bool().unwrap_or(false);
                let command = server["command"].as_str().unwrap_or("");

                // Only show non-builtin extensions that have an installation command
                if !is_builtin && !command.is_empty() {
                    available_extensions.push(format!(
                        "- {} ({}) - {}\n  Install with: {}",
                        name, id, description, command
                    ));
                }
            }

            if !available_extensions.is_empty() {
                output_parts.push(format!(
                    "\nAvailable extensions that user can install:\n{}\n",
                    available_extensions.join("\n")
                ));
            }
        }

        // Add a note about installation
        output_parts.push("\nRemind the user that to enable a disabled extension, they should use the Settings page in Goose or the CLI configure command to toggle the extension on.".to_string());
        output_parts.push("Remind the user that to install a new extension, they should use the Settings page in Goose or the CLI configure command with the relevant run command.".to_string());

        Ok(vec![Content::text(output_parts.join("\n"))])
    }

    async fn install_extension(&self, extension_name: String) -> Result<Vec<Content>, ToolError> {
        let extension = ExtensionManager::get_config(&extension_name)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get extension config: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionError(format!("Extension {} not found", extension_name)))?;

        // Create a real agent using AgentFactory
        let model_config = ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());
        let provider = Box::new(MockProvider { model_config }); // Still need mock provider
        // We want the agent from goose-cli to be used here
        let mut agent = AgentFactory::create(&AgentFactory::configured_version(), provider)
            .expect("Failed to create agent");

        // This will properly set up the extension through Capabilities
        agent.add_extension(extension)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to add extension: {}", e)))?;

        Ok(vec![Content::text(format!("Successfully installed and enabled extension '{}'", extension_name))])
    }
}

impl Router for GooseUtilsRouter {
    fn name(&self) -> String {
        "goose_utils".to_string()
    }

    fn instructions(&self) -> String {
        "Goose utilities for managing extensions and other common functionality.".to_string()
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
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "discover_extensions" => this.discover_extensions(arguments).await,
                "install_extension" => {
                    let extension_name = arguments
                        .get("extension_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing extension_name parameter".to_string())
                        })?
                        .to_string();
                    this.install_extension(extension_name).await
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
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

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt '{}' not found",
                prompt_name
            )))
        })
    }
}

impl Clone for GooseUtilsRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}

fn create_discover_extensions_tool() -> Tool {
    Tool::new(
        "discover_extensions".to_string(),
        "Discover additional capabilities to help complete tasks. Lists extensions that are available but not currently active. Use this tool when you're unable to find a specific feature or functionality, or when standard approaches aren't working. These extensions might provide the exact tools needed to solve your problem. If you find a relevant one, suggest that the user enable the extension.
        
        Also lists extensions curated by the Goose team that can be installed. To install them, direct the user to install them via the Goose Settings UI or the Goose CLI configure command with the command they will need to configure/add the extension. They cannot just enter the command directly into terminal to install. They will have to go through the CLI or the Settings UI outside of the current Goose session.
        
        You have a preference for suggesting the user enable any already-installed relevant extensions and otherwise installing the relevant extension.".to_string(),
        json!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
    )
}

// Create the tool definition
fn create_install_extension_tool() -> Tool {
    Tool::new(
        "install_extension".to_string(),
        "Install and enable a Goose extension. This tool takes an extension name and installs the extension \
         using information from the extension registry. The extension will be installed and enabled automatically. \
         Only non-builtin extensions can be installed this way.".to_string(),
        json!({
            "type": "object",
            "required": ["extension_name"],
            "properties": {
                "extension_name": {
                    "type": "string",
                    "description": "The name of the extension to install"
                }
            }
        }),
    )
}

// TODO: centralize MockProvider
#[derive(Clone)]
struct MockProvider {
    model_config: ModelConfig,
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::empty()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }

    async fn complete(
        &self,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
        Ok((
            Message::assistant().with_text("Mock response"),
            ProviderUsage::new("mock".to_string(), Usage::default()),
        ))
    }
}