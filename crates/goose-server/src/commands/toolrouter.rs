use anyhow::Result;
use goose::config::{ExtensionConfigManager, ExtensionEntry};
use goose::agents::extension::ExtensionConfig;

/// Register the ToolRouter as a built-in extension
pub async fn register_toolrouter() -> Result<()> {
    // Create the ToolRouter extension entry
    let entry = ExtensionEntry {
        enabled: true,
        config: ExtensionConfig::Builtin {
            name: "toolrouter".to_string(),
            display_name: Some("Tool Router".to_string()),
            timeout: None, // Use default timeout
            bundled: Some(true),
        },
    };
    
    // Register the extension
    ExtensionConfigManager::set(entry)?;
    
    Ok(())
}