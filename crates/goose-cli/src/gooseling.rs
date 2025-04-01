use anyhow::{Context, Result};
use console::style;
use std::path::Path;

use goose::gooselings::Gooseling;

/// Loads and validates a gooseling from a YAML file
///
/// # Arguments
///
/// * `path` - Path to the gooseling YAML file
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
/// - The YAML is invalid
/// - The required fields are missing
pub fn load_gooseling<P: AsRef<Path>>(path: P) -> Result<Gooseling> {
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

    // Parse YAML into Gooseling struct
    let gooseling: Gooseling = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse gooseling file: {}", path.display()))?;

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

    Ok(gooseling)
}
