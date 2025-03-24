#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    use crate::transport::{StdioTransport, WhitelistSource};

    #[test]
    fn test_default_behavior() {
        // By default, all commands should be allowed
        let transport = StdioTransport::new("any-command", vec![], HashMap::new());
        assert!(transport.is_command_allowed().is_ok());
        
        // Even commands that wouldn't normally be in a whitelist should be allowed
        let transport = StdioTransport::new("some-random-command-name", vec![], HashMap::new());
        assert!(transport.is_command_allowed().is_ok());
    }

    #[test]
    fn test_static_whitelist() {
        // Create a transport with a static whitelist
        let transport = StdioTransport::new(
            "custom-command", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::Static(vec!["custom-command".to_string()])
        );
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Create a transport with a command not in the static whitelist
        let transport = StdioTransport::new(
            "not-in-whitelist", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::Static(vec!["custom-command".to_string()])
        );
        
        assert!(transport.is_command_allowed().is_err());
    }

    #[test]
    fn test_allow_commands() {
        // Create a transport and add commands to the whitelist
        let transport = StdioTransport::new(
            "custom-command", 
            vec![], 
            HashMap::new()
        ).allow_commands(["custom-command", "another-command"]);
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with another allowed command
        let transport = StdioTransport::new(
            "another-command", 
            vec![], 
            HashMap::new()
        ).allow_commands(["custom-command", "another-command"]);
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with a command not in the whitelist
        let transport = StdioTransport::new(
            "not-in-whitelist", 
            vec![], 
            HashMap::new()
        ).allow_commands(["custom-command", "another-command"]);
        
        assert!(transport.is_command_allowed().is_err());
    }

    #[test]
    fn test_env_var_whitelist() {
        // Set an environment variable with a list of allowed commands
        env::set_var("TEST_ALLOWED_COMMANDS", "env-command,another-env-command");
        
        // Create a transport with the environment variable whitelist
        let transport = StdioTransport::new(
            "env-command", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::EnvVar("TEST_ALLOWED_COMMANDS".to_string())
        );
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with another allowed command
        let transport = StdioTransport::new(
            "another-env-command", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::EnvVar("TEST_ALLOWED_COMMANDS".to_string())
        );
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with a command not in the whitelist
        let transport = StdioTransport::new(
            "not-in-whitelist", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::EnvVar("TEST_ALLOWED_COMMANDS".to_string())
        );
        
        assert!(transport.is_command_allowed().is_err());
        
        // Clean up
        env::remove_var("TEST_ALLOWED_COMMANDS");
    }

    #[test]
    fn test_file_whitelist() {
        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("whitelist.txt");
        
        // Create a whitelist file
        let mut file = File::create(&file_path).expect("Failed to create whitelist file");
        writeln!(file, "file-command").expect("Failed to write to whitelist file");
        writeln!(file, "another-file-command").expect("Failed to write to whitelist file");
        writeln!(file, "# This is a comment").expect("Failed to write to whitelist file");
        file.flush().expect("Failed to flush whitelist file");
        
        // Create a transport with the file whitelist
        let transport = StdioTransport::new(
            "file-command", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::File(file_path.to_string_lossy().to_string())
        );
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with another allowed command
        let transport = StdioTransport::new(
            "another-file-command", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::File(file_path.to_string_lossy().to_string())
        );
        
        assert!(transport.is_command_allowed().is_ok());
        
        // Test with a command not in the whitelist
        let transport = StdioTransport::new(
            "not-in-whitelist", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::File(file_path.to_string_lossy().to_string())
        );
        
        assert!(transport.is_command_allowed().is_err());
        
        // Test with a comment line (should not be in whitelist)
        let transport = StdioTransport::new(
            "# This is a comment", 
            vec![], 
            HashMap::new()
        ).with_whitelist(
            WhitelistSource::File(file_path.to_string_lossy().to_string())
        );
        
        assert!(transport.is_command_allowed().is_err());
    }

    #[test]
    fn test_path_resolution() {
        // This test depends on the system having certain executables
        // We'll test with a command that should be available on all systems: the Rust compiler
        
        // First, make sure we can find the rust compiler
        let rustc_output = std::process::Command::new("rustc")
            .arg("--version")
            .output();
            
        // Only run the test if rustc is available
        if let Ok(_) = rustc_output {
            // Test with just the command name
            let transport = StdioTransport::new(
                "rustc", 
                vec![], 
                HashMap::new()
            ).allow_commands(["rustc"]);
            
            assert!(transport.is_command_allowed().is_ok());
            
            // Get the actual path to rustc
            if let Some(rustc_path) = transport.resolve_command_path() {
                println!("Resolved rustc path: {}", rustc_path);
                
                // Test with the full path but allowing just the command name
                let transport = StdioTransport::new(
                    &rustc_path, 
                    vec![], 
                    HashMap::new()
                ).allow_commands(["rustc"]);
                
                let result = transport.is_command_allowed();
                println!("Test with full path: {:?}", result);
                assert!(result.is_ok());
                
                // Test with the command name but allowing the full path
                let transport = StdioTransport::new(
                    "rustc", 
                    vec![], 
                    HashMap::new()
                ).allow_commands([&rustc_path]);
                
                let result = transport.is_command_allowed();
                println!("Test with command name: {:?}", result);
                assert!(result.is_ok());
            } else {
                println!("Could not resolve rustc path");
            }
        } else {
            println!("rustc not available, skipping test");
        }
    }
}