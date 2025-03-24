use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use super::{send_message, Error, PendingRequests, Transport, TransportHandle, TransportMessage};

/// A `StdioTransport` uses a child process's stdin/stdout as a communication channel.
///
/// It uses channels for message passing and handles responses asynchronously through a background task.
pub struct StdioActor {
    receiver: mpsc::Receiver<TransportMessage>,
    pending_requests: Arc<PendingRequests>,
    _process: Child, // we store the process to keep it alive
    error_sender: mpsc::Sender<Error>,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl StdioActor {
    pub async fn run(mut self) {
        use tokio::pin;

        let incoming = Self::handle_incoming_messages(self.stdout, self.pending_requests.clone());
        let outgoing = Self::handle_outgoing_messages(
            self.receiver,
            self.stdin,
            self.pending_requests.clone(),
        );

        // take ownership of futures for tokio::select
        pin!(incoming);
        pin!(outgoing);

        // Use select! to wait for either I/O completion or process exit
        tokio::select! {
            result = &mut incoming => {
                tracing::debug!("Stdin handler completed: {:?}", result);
            }
            result = &mut outgoing => {
                tracing::debug!("Stdout handler completed: {:?}", result);
            }
            // capture the status so we don't need to wait for a timeout
            status = self._process.wait() => {
                tracing::debug!("Process exited with status: {:?}", status);
            }
        }

        // Then always try to read stderr before cleaning up
        let mut stderr_buffer = Vec::new();
        if let Ok(bytes) = self.stderr.read_to_end(&mut stderr_buffer).await {
            let err_msg = if bytes > 0 {
                String::from_utf8_lossy(&stderr_buffer).to_string()
            } else {
                "Process ended unexpectedly".to_string()
            };

            tracing::info!("Process stderr: {}", err_msg);
            let _ = self
                .error_sender
                .send(Error::StdioProcessError(err_msg))
                .await;
        }

        // Clean up regardless of which path we took
        self.pending_requests.clear().await;
    }

    async fn handle_incoming_messages(stdout: ChildStdout, pending_requests: Arc<PendingRequests>) {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    tracing::error!("Child process ended (EOF on stdout)");
                    break;
                } // EOF
                Ok(_) => {
                    if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&line) {
                        tracing::debug!(
                            message = ?message,
                            "Received incoming message"
                        );

                        match &message {
                            JsonRpcMessage::Response(response) => {
                                if let Some(id) = &response.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            JsonRpcMessage::Error(error) => {
                                if let Some(id) = &error.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            _ => {} // TODO: Handle other variants (Request, etc.)
                        }
                    }
                    line.clear();
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Error reading line");
                    break;
                }
            }
        }
    }

    async fn handle_outgoing_messages(
        mut receiver: mpsc::Receiver<TransportMessage>,
        mut stdin: ChildStdin,
        pending_requests: Arc<PendingRequests>,
    ) {
        while let Some(mut transport_msg) = receiver.recv().await {
            let message_str = match serde_json::to_string(&transport_msg.message) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(tx) = transport_msg.response_tx.take() {
                        let _ = tx.send(Err(Error::Serialization(e)));
                    }
                    continue;
                }
            };

            tracing::debug!(message = ?transport_msg.message, "Sending outgoing message");

            if let Some(response_tx) = transport_msg.response_tx.take() {
                if let JsonRpcMessage::Request(request) = &transport_msg.message {
                    if let Some(id) = &request.id {
                        pending_requests.insert(id.to_string(), response_tx).await;
                    }
                }
            }

            if let Err(e) = stdin
                .write_all(format!("{}\n", message_str).as_bytes())
                .await
            {
                tracing::error!(error = ?e, "Error writing message to child process");
                pending_requests.clear().await;
                break;
            }

            if let Err(e) = stdin.flush().await {
                tracing::error!(error = ?e, "Error flushing message to child process");
                pending_requests.clear().await;
                break;
            }
        }
    }
}

#[derive(Clone)]
pub struct StdioTransportHandle {
    sender: mpsc::Sender<TransportMessage>,
    error_receiver: Arc<Mutex<mpsc::Receiver<Error>>>,
}

#[async_trait::async_trait]
impl TransportHandle for StdioTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error> {
        let result = send_message(&self.sender, message).await;
        // Check for any pending errors even if send is successful
        self.check_for_errors().await?;
        result
    }
}

impl StdioTransportHandle {
    /// Check if there are any process errors
    pub async fn check_for_errors(&self) -> Result<(), Error> {
        match self.error_receiver.lock().await.try_recv() {
            Ok(error) => {
                tracing::debug!("Found error: {:?}", error);
                Err(error)
            }
            Err(_) => Ok(()),
        }
    }
}

/// A `StdioTransport` uses a child process's stdin/stdout as a communication channel.
///
/// It uses channels for message passing and handles responses asynchronously through a background task.
/// For security, it includes an optional command whitelist system to restrict which commands can be executed.
///
/// # Security
///
/// By default, all commands are allowed to be executed. To restrict which commands can be executed,
/// you can configure a whitelist using one of the following methods:
///
/// - `with_whitelist`: Set a custom whitelist source (static list, environment variable, or file)
/// - `allow_commands`: Add specific commands to the whitelist
///
/// If a whitelist is configured and a command is not in the whitelist, the `spawn_process` method
/// will return an error.
///
/// # Examples
///
/// ```
/// use mcp_client::transport::{StdioTransport, WhitelistSource};
///
/// // Create a transport with no whitelist (all commands allowed)
/// let transport = StdioTransport::new("python", vec!["-c".to_string(), "print('hello')".to_string()], Default::default());
///
/// // Create a transport with a whitelist
/// let transport = StdioTransport::new("custom-tool", vec![], Default::default())
///     .allow_commands(["custom-tool", "another-tool"]);
/// ```
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    whitelist_source: Option<WhitelistSource>,
}

/// Source for the command whitelist
///
/// This enum defines different ways to specify allowed commands for process execution.
/// By default (if no whitelist source is specified), all commands are allowed.
/// 
/// # Examples
///
/// ```
/// use mcp_client::transport::{StdioTransport, WhitelistSource};
/// 
/// // Using a static list of commands
/// let transport = StdioTransport::new("python", vec!["-c".to_string(), "print('hello')".to_string()], Default::default())
///     .with_whitelist(WhitelistSource::Static(vec!["python".to_string(), "node".to_string()]));
///
/// // Using an environment variable
/// let transport = StdioTransport::new("python", vec![], Default::default())
///     .with_whitelist(WhitelistSource::EnvVar("MY_ALLOWED_COMMANDS".to_string()));
///
/// // Using a file
/// let transport = StdioTransport::new("python", vec![], Default::default())
///     .with_whitelist(WhitelistSource::File("/path/to/allowed_commands.txt".to_string()));
///
/// // Adding specific commands
/// let transport = StdioTransport::new("python", vec![], Default::default())
///     .allow_commands(["python", "node", "npm"]);
/// ```
pub enum WhitelistSource {
    /// A static list of allowed commands
    Static(Vec<String>),
    /// Environment variable name containing comma-separated commands
    EnvVar(String),
    /// Path to a file containing allowed commands (one per line)
    File(String),
}

impl StdioTransport {
    pub fn new<S: Into<String>>(
        command: S,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            env,
            whitelist_source: None,
        }
    }

    /// Set a custom whitelist source
    pub fn with_whitelist(mut self, source: WhitelistSource) -> Self {
        self.whitelist_source = Some(source);
        self
    }
    
    /// Add specific commands to the whitelist
    /// 
    /// This creates a static whitelist if none exists, or adds to an existing static whitelist.
    /// If a different type of whitelist source was previously set, it will be replaced.
    pub fn allow_commands<I, S>(mut self, commands: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands_vec: Vec<String> = commands.into_iter().map(|s| s.into()).collect();
        
        match &mut self.whitelist_source {
            Some(WhitelistSource::Static(existing)) => {
                // Add to existing static whitelist
                existing.extend(commands_vec);
            }
            _ => {
                // Create a new static whitelist
                self.whitelist_source = Some(WhitelistSource::Static(commands_vec));
            }
        }
        
        self
    }

    /// Checks if the command is in the allowed whitelist
    /// 
    /// This method is used internally before spawning a process, but is also
    /// exposed for testing purposes.
    /// 
    /// If no whitelist is configured (empty whitelist), all commands are allowed.
    pub fn is_command_allowed(&self) -> Result<(), Error> {
        // Get the dynamic whitelist from environment or configuration
        let whitelist = self.get_command_whitelist();
        
        // If the whitelist is empty, all commands are allowed
        if whitelist.is_empty() {
            tracing::debug!("Command '{}' is allowed (no whitelist configured)", self.command);
            return Ok(());
        }
        
        tracing::debug!("Checking command '{}' against whitelist", self.command);
        
        // Extract the command name without path
        let command_name = std::path::Path::new(&self.command)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.command);
            
        // Check if the command name (without path) is in the whitelist
        if whitelist.contains(&command_name.to_string()) {
            tracing::debug!("Command '{}' is allowed (name match: '{}')", self.command, command_name);
            return Ok(());
        }
        
        // Check if the command (with full path) is in the whitelist
        if whitelist.contains(&self.command) {
            tracing::debug!("Command '{}' is allowed (direct match)", self.command);
            return Ok(());
        }
        
        // If not found by name, try to resolve the absolute path and check again
        if let Some(abs_path) = self.resolve_command_path() {
            tracing::debug!("Resolved command '{}' to path '{}'", self.command, abs_path);
            if whitelist.contains(&abs_path) {
                tracing::debug!("Command '{}' is allowed (path match: '{}')", self.command, abs_path);
                return Ok(());
            }
            
            // Extract the command name from the resolved path and check again
            let resolved_name = std::path::Path::new(&abs_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&abs_path);
                
            if whitelist.contains(&resolved_name.to_string()) {
                tracing::debug!("Command '{}' is allowed (resolved name match: '{}')", self.command, resolved_name);
                return Ok(());
            }
        }
        
        // Command is not allowed
        tracing::warn!("Command '{}' is not in the allowed whitelist", self.command);
        Err(Error::StdioProcessError(format!(
            "Command '{}' is not in the allowed whitelist",
            self.command
        )))
    }
    
    /// Resolve the absolute path of the command
    /// 
    /// This method attempts to find the full path of a command by checking
    /// the PATH environment variable. It is exposed for testing purposes.
    pub fn resolve_command_path(&self) -> Option<String> {
        use std::path::Path;
        
        // If the command is already an absolute path, return it
        if Path::new(&self.command).is_absolute() {
            return Some(self.command.clone());
        }
        
        // Try to find the command in PATH
        if let Ok(path_var) = std::env::var("PATH") {
            let paths = std::env::split_paths(&path_var);
            for dir in paths {
                let full_path = dir.join(&self.command);
                if full_path.exists() {
                    if let Some(path_str) = full_path.to_str() {
                        return Some(path_str.to_string());
                    }
                }
            }
        }
        
        None
    }

    /// Get the dynamic whitelist of allowed commands
    /// 
    /// This method returns the list of allowed commands based on the configured whitelist source.
    /// If no whitelist source is configured, returns an empty vector which signals that all commands
    /// are allowed.
    pub fn get_command_whitelist(&self) -> Vec<String> {
        // Check if a custom whitelist source was provided
        if let Some(source) = &self.whitelist_source {
            match source {
                WhitelistSource::Static(commands) => {
                    tracing::debug!("Using static whitelist with {} commands", commands.len());
                    return commands.clone();
                }
                WhitelistSource::EnvVar(var_name) => {
                    if let Ok(allowed_cmds) = std::env::var(var_name) {
                        let commands: Vec<String> = allowed_cmds
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                        tracing::debug!(
                            "Using whitelist from environment variable '{}' with {} commands",
                            var_name, commands.len()
                        );
                        return commands;
                    } else {
                        tracing::warn!("Environment variable '{}' not found, all commands will be allowed", var_name);
                    }
                }
                WhitelistSource::File(path) => {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let commands: Vec<String> = content
                            .lines()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty() && !s.starts_with('#'))
                            .collect();
                        tracing::debug!(
                            "Using whitelist from file '{}' with {} commands", 
                            path, commands.len()
                        );
                        return commands;
                    } else {
                        tracing::warn!("Could not read whitelist file '{}', all commands will be allowed", path);
                    }
                }
            }
        }
        
        // If no whitelist source is configured or if the configured source failed,
        // return an empty vector (which means all commands are allowed)
        tracing::debug!("No whitelist configured, all commands are allowed");
        Vec::new()
    }

    async fn spawn_process(&self) -> Result<(Child, ChildStdin, ChildStdout, ChildStderr), Error> {
        // Check against whitelist before proceeding
        self.is_command_allowed()?;
        
        let mut command = Command::new(&self.command);
        command
            .envs(&self.env)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        // Set process group only on Unix systems
        #[cfg(unix)]
        command.process_group(0); // don't inherit signal handling from parent process

        // Hide console window on Windows
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW flag

        let mut process = command
            .spawn()
            .map_err(|e| Error::StdioProcessError(e.to_string()))?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stdin".into()))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stdout".into()))?;

        let stderr = process
            .stderr
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stderr".into()))?;

        Ok((process, stdin, stdout, stderr))
    }
}

#[async_trait]
impl Transport for StdioTransport {
    type Handle = StdioTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        let (process, stdin, stdout, stderr) = self.spawn_process().await?;
        let (message_tx, message_rx) = mpsc::channel(32);
        let (error_tx, error_rx) = mpsc::channel(1);

        let actor = StdioActor {
            receiver: message_rx,
            pending_requests: Arc::new(PendingRequests::new()),
            _process: process,
            error_sender: error_tx,
            stdin,
            stdout,
            stderr,
        };

        tokio::spawn(actor.run());

        let handle = StdioTransportHandle {
            sender: message_tx,
            error_receiver: Arc::new(Mutex::new(error_rx)),
        };
        Ok(handle)
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}
