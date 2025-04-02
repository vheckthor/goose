use anyhow::{Context, Result};
use console::style;
use std::path::Path;

use goose::gooselings::Gooseling;

/// Loads and validates a gooseling from a YAML or JSON file
///
/// # Arguments
///
/// * `path` - Path to the gooseling file (YAML or JSON)
///
/// # Returns
///
/// The parsed Gooseling struct if successful
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - The file can't be read
/// - The YAML/JSON is invalid
/// - The required fields are missing
pub fn load_gooseling<P: AsRef<Path>>(path: P, log: bool) -> Result<Gooseling> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "Gooseling file not found: {}",
            path.display()
        ));
    }
    // Read file content
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read gooseling file: {}", path.display()))?;

    // Determine file format based on extension and parse accordingly
    let gooseling: Gooseling = if let Some(extension) = path.extension() {
        match extension.to_str().unwrap_or("").to_lowercase().as_str() {
            "json" => serde_json::from_str(&content).with_context(|| {
                format!("Failed to parse JSON gooseling file: {}", path.display())
            })?,
            "yaml" | "yml" => serde_yaml::from_str(&content).with_context(|| {
                format!("Failed to parse YAML gooseling file: {}", path.display())
            })?,
            _ => {
                return Err(anyhow::anyhow!(
                "Unsupported file format for gooseling file: {}. Expected .yaml, .yml, or .json",
                path.display()
            ))
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "File has no extension: {}. Expected .yaml, .yml, or .json",
            path.display()
        ));
    };

    if log {
        // Display information about the loaded gooseling
        println!(
            "{} {}",
            style("Loading gooseling:").green().bold(),
            style(&gooseling.title).green()
        );
        println!("{} {}", style("Description:").dim(), &gooseling.description);

        // Display activities if available
        if let Some(activities) = &gooseling.activities {
            println!("\n{}:", style("Activities").dim());
            for activity in activities {
                println!("- {}", activity);
            }
        }

        println!(); // Add a blank line for spacing
    }

    Ok(gooseling)
}
