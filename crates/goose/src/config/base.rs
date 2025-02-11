use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use keyring::Entry;
use once_cell::sync::{Lazy, OnceCell};
use crate::config_manager::ConfigManager;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

const KEYRING_SERVICE: &str = "goose";
const KEYRING_USERNAME: &str = "secrets";

#[cfg(test)]
const TEST_KEYRING_SERVICE: &str = "goose-test";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration value not found: {0}")]
    NotFound(String),
    #[error("Failed to deserialize value: {0}")]
    DeserializeError(String),
    #[error("Failed to read config file: {0}")]
    FileError(#[from] std::io::Error),
    #[error("Failed to create config directory: {0}")]
    DirectoryError(String),
    #[error("Failed to access keyring: {0}")]
    KeyringError(String),
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::DeserializeError(err.to_string())
    }
}
use std::path::Path;

#[derive(Debug)]
pub struct Config {
    manager: ConfigManager,
}

impl Default for Config {
    fn default() -> Self {
        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        let config_dir = choose_app_strategy(APP_STRATEGY.clone())
            .expect("goose requires a home dir")
            .config_dir();

        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
            .join(".config")
            .join("goose");

        Config {
            manager: ConfigManager::new("goose", config_dir.to_str().unwrap()),
        }
    }
}

impl Config {
    pub fn global() -> &'static ConfigManager {
        lazy_static! {
            static ref CONFIG_MANAGER: ConfigManager = ConfigManager::new(
                "goose",
                dirs::home_dir()
                    .expect("goose requires a home dir")
                    .join(".config")
                    .join("goose")
                    .to_str()
                    .unwrap(),
            );
        }
        &CONFIG_MANAGER
    }

    pub fn new<P: AsRef<Path>>(config_path: P, service: &str) -> ConfigManager {
        ConfigManager::new(service, config_path.as_ref().to_str().unwrap())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(key: &str) -> Result<T, String> {
        Config::global().get(key).map_err(|e| e.to_string())
    }

    pub fn set(key: &str, value: Value) -> Result<(), String> {
        Config::global().set(key, value).map_err(|e| e.to_string())
    }

    pub fn delete(key: &str) -> Result<(), String> {
        Config::global().delete(key).map_err(|e| e.to_string())
    }

    pub fn get_secret<T: for<'de> Deserialize<'de>>(key: &str) -> Result<T, String> {
        Config::global().get_secret(key).map_err(|e| e.to_string())
    }

    pub fn set_secret(key: &str, value: Value) -> Result<(), String> {
        Config::global().set_secret(key, value).map_err(|e| e.to_string())
    }

    pub fn delete_secret(key: &str) -> Result<(), String> {
        Config::global().delete_secret(key).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::NamedTempFile;

    #[test]
    fn test_basic_config() -> Result<(), String> {
        let temp_file = NamedTempFile::new().unwrap();
        let config_manager = ConfigManager::new("goose-test", temp_file.path().to_str().unwrap());

        // Set a simple string value
        config_manager.set("test_key", Value::String("test_value".to_string()))?;

        // Test simple string retrieval
        let value: String = config_manager.get("test_key")?;
        assert_eq!(value, "test_value");

        Ok(())
    }
}
