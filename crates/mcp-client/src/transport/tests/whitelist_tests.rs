use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

use crate::transport::StdioTransport;

#[test]
fn test_allowlist() {
    // Create a temporary directory
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("allowed_extensions.yaml");

    // Create a whitelist file
    let mut file = File::create(&file_path).expect("Failed to create allowlist file");
    writeln!(file, "extensions:").expect("Failed to write to allowlist file");
    writeln!(file, "  - python").expect("Failed to write to allowlist file");
    writeln!(file, "  - node").expect("Failed to write to allowlist file");
    file.flush().expect("Failed to flush allowlist file");

    // Set the environment variable
    env::set_var(
        "GOOSE_MCP_ALLOWLIST",
        file_path.to_string_lossy().to_string(),
    );

    // Test with an allowed command
    let transport = StdioTransport::new("python", vec![], HashMap::new());
    assert!(transport.is_command_allowed().is_ok());

    // Test with another allowed command
    let transport = StdioTransport::new("node", vec![], HashMap::new());
    assert!(transport.is_command_allowed().is_ok());

    // Test with a command not in the allowlist
    let transport = StdioTransport::new("not-in-allowlist", vec![], HashMap::new());
    assert!(transport.is_command_allowed().is_err());

    // Clean up
    env::remove_var("GOOSE_MCP_ALLOWLIST");
}

#[test]
fn test_no_allowlist() {
    // Make sure the environment variable is not set
    env::remove_var("GOOSE_MCP_ALLOWLIST");

    // Without an allowlist, all commands should be allowed
    let transport = StdioTransport::new("any-command", vec![], HashMap::new());
    assert!(transport.is_command_allowed().is_ok());
}
