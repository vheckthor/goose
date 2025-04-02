use anyhow::Result;
use base64::Engine;
use console::style;
use std::path::Path;

use crate::gooseling::load_gooseling;

/// Validates a gooseling file
///
/// # Arguments
///
/// * `file_path` - Path to the gooseling file to validate
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_validate<P: AsRef<Path>>(file_path: P) -> Result<()> {
    // Load and validate the gooseling file
    match load_gooseling(&file_path, false) {
        Ok(_) => {
            println!("{} Gooseling file is valid", style("✓").green().bold());
            Ok(())
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}

/// Generates a deeplink for a gooseling file
///
/// # Arguments
///
/// * `file_path` - Path to the gooseling file
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_deeplink<P: AsRef<Path>>(file_path: P) -> Result<()> {
    // Load the gooseling file first to validate it
    match load_gooseling(&file_path, false) {
        Ok(gooseling) => {
            if let Ok(gooseling_json) = serde_json::to_string(&gooseling) {
                let deeplink = base64::engine::general_purpose::STANDARD.encode(gooseling_json);
                println!(
                    "{} Generated deeplink for: {}",
                    style("✓").green().bold(),
                    gooseling.title
                );
                println!("goose://gooseling?config={}", deeplink);
            }
            Ok(())
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}
