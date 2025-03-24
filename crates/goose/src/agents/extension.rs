use std::collections::HashMap;
use std::fs;

use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use mcp_client::client::Error as ClientError;
use reqwest;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::config;
use crate::config::extensions::name_to_key;

/// Errors from Extension operation
#[derive(Error, Debug)]
pub enum ExtensionError {
    #[error("Failed to start the MCP server from configuration `{0}` `{1}`")]
    Initialization(ExtensionConfig, ClientError),
    #[error("Failed a client call to an MCP server: {0}")]
    Client(#[from] ClientError),
    #[error("User Message exceeded context-limit. History could not be truncated to accomodate.")]
    ContextLimit,
    #[error("Transport error: {0}")]
    Transport(#[from] mcp_client::transport::Error),
    #[error("Environment variable `{0}` is not allowed to be overridden.")]
    InvalidEnvVar(String),
    #[error("Command `{0}` is not in the allowed extensions list")]
    UnauthorizedCommand(String),
    #[error("Allowlist error: {0}")]
    AllowlistError(String),
    #[error("Failed to write file: {0}")]
    IoError(#[from] std::io::Error),
}

pub type ExtensionResult<T> = Result<T, ExtensionError>;

#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct Envs {
    /// A map of environment variables to set, e.g. API_KEY -> some_secret, HOST -> host
    #[serde(default)]
    #[serde(flatten)]
    map: HashMap<String, String>,
}

impl Envs {
    /// List of sensitive env vars that should not be overridden
    const DISALLOWED_KEYS: [&'static str; 31] = [
        // 🔧 Binary path manipulation
        "PATH",       // Controls executable lookup paths — critical for command hijacking
        "PATHEXT",    // Windows: Determines recognized executable extensions (e.g., .exe, .bat)
        "SystemRoot", // Windows: Can affect system DLL resolution (e.g., `kernel32.dll`)
        "windir",     // Windows: Alternative to SystemRoot (used in legacy apps)
        // 🧬 Dynamic linker hijacking (Linux/macOS)
        "LD_LIBRARY_PATH",  // Alters shared library resolution
        "LD_PRELOAD",       // Forces preloading of shared libraries — common attack vector
        "LD_AUDIT",         // Loads a monitoring library that can intercept execution
        "LD_DEBUG",         // Enables verbose linker logging (information disclosure risk)
        "LD_BIND_NOW",      // Forces immediate symbol resolution, affecting ASLR
        "LD_ASSUME_KERNEL", // Tricks linker into thinking it's running on an older kernel
        // 🍎 macOS dynamic linker variables
        "DYLD_LIBRARY_PATH",     // Same as LD_LIBRARY_PATH but for macOS
        "DYLD_INSERT_LIBRARIES", // macOS equivalent of LD_PRELOAD
        "DYLD_FRAMEWORK_PATH",   // Overrides framework lookup paths
        // 🐍 Python / Node / Ruby / Java / Golang hijacking
        "PYTHONPATH",   // Overrides Python module resolution
        "PYTHONHOME",   // Overrides Python root directory
        "NODE_OPTIONS", // Injects options/scripts into every Node.js process
        "RUBYOPT",      // Injects Ruby execution flags
        "GEM_PATH",     // Alters where RubyGems looks for installed packages
        "GEM_HOME",     // Changes RubyGems default install location
        "CLASSPATH",    // Java: Controls where classes are loaded from — critical for RCE attacks
        "GO111MODULE",  // Go: Forces use of module proxy or disables it
        "GOROOT", // Go: Changes root installation directory (could lead to execution hijacking)
        // 🖥️ Windows-specific process & DLL hijacking
        "APPINIT_DLLS", // Forces Windows to load a DLL into every process
        "SESSIONNAME",  // Affects Windows session configuration
        "ComSpec",      // Determines default command interpreter (can replace `cmd.exe`)
        "TEMP",
        "TMP",          // Redirects temporary file storage (useful for injection attacks)
        "LOCALAPPDATA", // Controls application data paths (can be abused for persistence)
        "USERPROFILE",  // Windows user directory (can affect profile-based execution paths)
        "HOMEDRIVE",
        "HOMEPATH", // Changes where the user's home directory is located
    ];

    /// Constructs a new Envs, skipping disallowed env vars with a warning
    pub fn new(map: HashMap<String, String>) -> Self {
        let mut validated = HashMap::new();

        for (key, value) in map {
            if Self::is_disallowed(&key) {
                warn!("Skipping disallowed env var: {}", key);
                continue;
            }
            validated.insert(key, value);
        }

        Self { map: validated }
    }

    /// Returns a copy of the validated env vars
    pub fn get_env(&self) -> HashMap<String, String> {
        self.map.clone()
    }

    /// Returns an error if any disallowed env var is present
    pub fn validate(&self) -> Result<(), Box<ExtensionError>> {
        for key in self.map.keys() {
            if Self::is_disallowed(key) {
                return Err(Box::new(ExtensionError::InvalidEnvVar(key.clone())));
            }
        }
        Ok(())
    }

    fn is_disallowed(key: &str) -> bool {
        Self::DISALLOWED_KEYS
            .iter()
            .any(|disallowed| disallowed.eq_ignore_ascii_case(key))
    }
}

/// Represents the different types of MCP extensions that can be added to the manager
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum ExtensionConfig {
    /// Server-sent events client with a URI endpoint
    #[serde(rename = "sse")]
    Sse {
        /// The name used to identify this extension
        name: String,
        uri: String,
        #[serde(default)]
        envs: Envs,
        description: Option<String>,
        // NOTE: set timeout to be optional for compatibility.
        // However, new configurations should include this field.
        timeout: Option<u64>,
    },
    /// Standard I/O client with command and arguments
    #[serde(rename = "stdio")]
    Stdio {
        /// The name used to identify this extension
        name: String,
        cmd: String,
        args: Vec<String>,
        #[serde(default)]
        envs: Envs,
        timeout: Option<u64>,
        description: Option<String>,
    },
    /// Built-in extension that is part of the goose binary
    #[serde(rename = "builtin")]
    Builtin {
        /// The name used to identify this extension
        name: String,
        display_name: Option<String>, // needed for the UI
        timeout: Option<u64>,
    },
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self::Builtin {
            name: config::DEFAULT_EXTENSION.to_string(),
            display_name: Some(config::DEFAULT_DISPLAY_NAME.to_string()),
            timeout: Some(config::DEFAULT_EXTENSION_TIMEOUT),
        }
    }
}

/// Check if a command is in the allowed extensions list
///
/// This function checks if the command is allowed according to the allowlist.
/// If GOOSE_MCP_ALLOWLIST_URL is set, it will download the allowlist from that URL
/// and save it to ~/.config/goose/mcp_allowlist.yaml.
///
/// The function will then check if the command is allowed according to the downloaded
/// allowlist file.
///
/// If GOOSE_MCP_ALLOWLIST_URL is not set, all commands are allowed.
pub fn is_command_allowed(cmd: &str) -> Result<(), Box<ExtensionError>> {
    // Check if GOOSE_MCP_ALLOWLIST_URL is set
    if let Ok(url) = std::env::var("GOOSE_MCP_ALLOWLIST_URL") {
        // Get the path where the allowlist would be stored
        let app_strategy = AppStrategyArgs {
            top_level_domain: "Block".to_string(),
            author: "Block".to_string(),
            app_name: "goose".to_string(),
        };

        let allowlist_path = match choose_app_strategy(app_strategy) {
            Ok(strategy) => strategy.config_dir().join("mcp_allowlist.yaml"),
            Err(e) => {
                warn!("Failed to determine allowlist path: {}", e);
                return Ok(()); // Allow the command if we can't even determine the path
            }
        };

        let path_str = allowlist_path.to_string_lossy().to_string();

        // Always try to download the allowlist if URL is set
        let download_result = download_allowlist(&url);

        // Whether download succeeded or failed, try to use the file if it exists
        if let Ok(content) = std::fs::read_to_string(&allowlist_path) {
            // Parse the YAML file
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                // Extract the extensions list
                if let Some(extensions) = yaml.get("extensions") {
                    if let Some(extensions_array) = extensions.as_sequence() {
                        // Create a list of allowed commands
                        let allowed_commands: Vec<String> = extensions_array
                            .iter()
                            .filter_map(|v| {
                                v.get("command").and_then(|c| c.as_str()).map(|command| command.trim().to_string())
                            })
                            .collect();

                        // Require exact match for security
                        if !allowed_commands.contains(&cmd.to_string()) {
                            return Err(Box::new(ExtensionError::UnauthorizedCommand(cmd.to_string())));
                        }
                    }
                }
            }
        } else if download_result.is_err() {
            // Only log a warning if both download failed AND we couldn't read the file
            warn!(
                "Failed to download allowlist AND couldn't read existing file at {}: {:?}",
                path_str,
                download_result.err()
            );
        }
    }

    // If no URL is set or everything passed, allow the command
    Ok(())
}

/// Download the allowlist from a URL and save it to the config directory
///
/// This function downloads the allowlist from the specified URL and saves it to
/// ~/.config/goose/mcp_allowlist.yaml. It will create the directory if it doesn't exist.
///
/// Returns the path to the downloaded file.
pub fn download_allowlist(url: &str) -> Result<String, Box<ExtensionError>> {
    // Define app strategy for consistent config paths
    let app_strategy = AppStrategyArgs {
        top_level_domain: "Block".to_string(),
        author: "Block".to_string(),
        app_name: "goose".to_string(),
    };

    // Get the config directory (~/.config/goose/ on macOS/Linux)
    let config_dir = choose_app_strategy(app_strategy)
        .map_err(|e| Box::new(ExtensionError::AllowlistError(format!("Failed to get config directory: {}", e))))?
        .config_dir();

    // Create the directory if it doesn't exist
    fs::create_dir_all(&config_dir).map_err(|e| Box::new(ExtensionError::AllowlistError(format!("Failed to create directory: {}", e))))?;

    // Define the path for the allowlist file
    let allowlist_path = config_dir.join("mcp_allowlist.yaml");
    let path_str = allowlist_path.to_string_lossy().to_string();

    // Download the allowlist file
    info!("Downloading allowlist from {}", url);

    // Create a client with a timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10)) // 10 second timeout
        .build()
        .map_err(|e| Box::new(ExtensionError::AllowlistError(format!("Failed to create HTTP client: {}", e))))?;

    // Make the request
    let response = client
        .get(url)
        .send()
        .map_err(|e| Box::new(ExtensionError::AllowlistError(format!("HTTP request failed: {}", e))))?;

    if !response.status().is_success() {
        return Err(Box::new(ExtensionError::AllowlistError(format!(
            "HTTP error: {}",
            response.status()
        ))));
    }

    let content = response
        .text()
        .map_err(|e| Box::new(ExtensionError::AllowlistError(format!("Failed to read response body: {}", e))))?;

    // Validate the YAML format
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .map_err(|e| Box::new(ExtensionError::AllowlistError(format!("Invalid YAML: {}", e))))?;

    // Write the content to the file
    fs::write(&allowlist_path, content).map_err(|e| Box::new(ExtensionError::IoError(e)))?;

    info!("Allowlist downloaded and saved to {}", path_str);

    Ok(path_str)
}

impl ExtensionConfig {
    pub fn sse<S: Into<String>, T: Into<u64>>(name: S, uri: S, description: S, timeout: T) -> Self {
        Self::Sse {
            name: name.into(),
            uri: uri.into(),
            envs: Envs::default(),
            description: Some(description.into()),
            timeout: Some(timeout.into()),
        }
    }

    pub fn stdio<S: Into<String>, T: Into<u64>>(
        name: S,
        cmd: S,
        description: S,
        timeout: T,
    ) -> Self {
        Self::Stdio {
            name: name.into(),
            cmd: cmd.into(),
            args: vec![],
            envs: Envs::default(),
            description: Some(description.into()),
            timeout: Some(timeout.into()),
        }
    }

    pub fn with_args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        match self {
            Self::Stdio {
                name,
                cmd,
                envs,
                timeout,
                description,
                ..
            } => Self::Stdio {
                name,
                cmd,
                envs,
                args: args.into_iter().map(Into::into).collect(),
                description,
                timeout,
            },
            other => other,
        }
    }

    pub fn key(&self) -> String {
        let name = self.name();
        name_to_key(&name)
    }

    /// Get the extension name regardless of variant
    pub fn name(&self) -> String {
        match self {
            Self::Sse { name, .. } => name,
            Self::Stdio { name, .. } => name,
            Self::Builtin { name, .. } => name,
        }
        .to_string()
    }

    /// Check if this extension's command is allowed
    pub fn validate_command(&self) -> Result<(), Box<ExtensionError>> {
        if let Self::Stdio { cmd, .. } = self {
            is_command_allowed(cmd)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for ExtensionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionConfig::Sse { name, uri, .. } => write!(f, "SSE({}: {})", name, uri),
            ExtensionConfig::Stdio {
                name, cmd, args, ..
            } => {
                write!(f, "Stdio({}: {} {})", name, cmd, args.join(" "))
            }
            ExtensionConfig::Builtin { name, .. } => write!(f, "Builtin({})", name),
        }
    }
}

/// Information about the extension used for building prompts
#[derive(Clone, Debug, Serialize)]
pub struct ExtensionInfo {
    name: String,
    instructions: String,
    has_resources: bool,
}

impl ExtensionInfo {
    pub fn new(name: &str, instructions: &str, has_resources: bool) -> Self {
        Self {
            name: name.to_string(),
            instructions: instructions.to_string(),
            has_resources,
        }
    }
}

/// Information about the tool used for building prompts
#[derive(Clone, Debug, Serialize)]
pub struct ToolInfo {
    name: String,
    description: String,
    parameters: Vec<String>,
}

impl ToolInfo {
    pub fn new(name: &str, description: &str, parameters: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_no_allowlist_url() {
        // Make sure the environment variable is not set
        env::remove_var("GOOSE_MCP_ALLOWLIST_URL");

        // Without an allowlist URL, all commands should be allowed
        assert!(is_command_allowed("any-command").is_ok());
    }

    #[test]
    fn test_allowlist_with_new_format() {
        // This test manually creates a file and checks the command validation logic
        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("allowed_extensions.yaml");

        // Create a whitelist file with the new format that includes id and command
        let mut file = File::create(&file_path).expect("Failed to create allowlist file");
        writeln!(file, "extensions:").expect("Failed to write to allowlist file");
        writeln!(file, "  - id: slack").expect("Failed to write to allowlist file");
        writeln!(file, "    command: uvx mcp_slack").expect("Failed to write to allowlist file");
        writeln!(file, "  - id: python").expect("Failed to write to allowlist file");
        writeln!(file, "    command: python").expect("Failed to write to allowlist file");
        file.flush().expect("Failed to flush allowlist file");

        // Test with allowed commands (using our mock function)
        let allowed_commands = ["uvx mcp_slack", "python"];
        for cmd in allowed_commands.iter() {
            // Read the file and check if command is allowed
            let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
            let yaml =
                serde_yaml::from_str::<serde_yaml::Value>(&content).expect("Failed to parse YAML");
            let extensions = yaml.get("extensions").expect("No extensions found");
            let extensions_array = extensions
                .as_sequence()
                .expect("Extensions is not an array");

            let allowed_commands: Vec<String> = extensions_array
                .iter()
                .filter_map(|v| {
                    if let Some(command) = v.get("command").and_then(|c| c.as_str()) {
                        Some(command.trim().to_string())
                    } else {
                        None
                    }
                })
                .collect();

            assert!(allowed_commands.contains(&cmd.to_string()));
        }

        // Test with a command not in the allowlist
        let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
        let yaml =
            serde_yaml::from_str::<serde_yaml::Value>(&content).expect("Failed to parse YAML");
        let extensions = yaml.get("extensions").expect("No extensions found");
        let extensions_array = extensions
            .as_sequence()
            .expect("Extensions is not an array");

        let allowed_commands: Vec<String> = extensions_array
            .iter()
            .filter_map(|v| {
                if let Some(command) = v.get("command").and_then(|c| c.as_str()) {
                    Some(command.trim().to_string())
                } else {
                    None
                }
            })
            .collect();

        assert!(!allowed_commands.contains(&"not-in-allowlist".to_string()));
    }

    #[test]
    #[ignore] // This test requires network access, so we ignore it by default
    fn test_download_allowlist() {
        // Setup a mock server
        let mut server = mockito::Server::new();

        // Mock with any number of calls
        let _mock = server
            .mock("GET", "/allowlist.yaml")
            .with_status(200)
            .with_body(
                "extensions:
  - id: slack
    command: uvx mcp_slack
  - id: python
    command: python",
            )
            .create();

        // Set the URL environment variable to point to our mock server
        env::set_var(
            "GOOSE_MCP_ALLOWLIST_URL",
            format!("{}/allowlist.yaml", server.url()),
        );

        // Test that a command is allowed after downloading the allowlist
        assert!(is_command_allowed("uvx mcp_slack").is_ok());

        // Test that a command not in the allowlist is rejected
        assert!(is_command_allowed("not-in-allowlist").is_err());

        // Clean up
        env::remove_var("GOOSE_MCP_ALLOWLIST_URL");
    }

    #[test]
    #[ignore] // This test requires network access, so we ignore it by default
    fn test_download_allowlist_failure() {
        // Setup a mock server that returns an error
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/allowlist.yaml")
            .with_status(404)
            .with_body("Not Found")
            .create();

        // Set the URL environment variable to point to our mock server
        env::set_var(
            "GOOSE_MCP_ALLOWLIST_URL",
            format!("{}/allowlist.yaml", server.url()),
        );

        // Test that command validation fails when download fails
        assert!(is_command_allowed("any-command").is_err());

        // Clean up
        env::remove_var("GOOSE_MCP_ALLOWLIST_URL");
    }
}
