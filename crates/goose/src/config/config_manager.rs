use keyring::Entry;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const KEYRING_USERNAME: &str = "secrets";

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

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::DeserializeError(err.to_string())
    }
}

impl From<keyring::Error> for ConfigError {
    fn from(err: keyring::Error) -> Self {
        ConfigError::KeyringError(err.to_string())
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    keyring_service: String,
}

impl ConfigManager {
    pub fn new(service: &str, config_dir: &str) -> Self {
        let config_path = Path::new(config_dir).join("config.yaml");
        fs::create_dir_all(&config_dir).expect("Failed to create config directory");

        ConfigManager {
            config_path,
            keyring_service: service.to_string(),
        }
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        self.get_from_env(key).or_else(|| self.get_from_file(key))
    }

    fn get_from_env<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        let env_key = key.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            let value: Value = serde_json::from_str(&val).unwrap_or(Value::String(val));
            return Ok(serde_json::from_value(value)?);
        }
        Err(ConfigError::NotFound(key.to_string()))
    }

    fn get_from_file<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        if self.config_path.exists() {
            let file_content = fs::read_to_string(&self.config_path)?;
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
            let json_value: Value = serde_json::to_value(yaml_value)?;

            if let Value::Object(map) = json_value {
                if let Some(v) = map.get(key) {
                    return Ok(serde_json::from_value(v.clone())?);
                }
            }
        }
        Err(ConfigError::NotFound(key.to_string()))
    }

    pub fn set(&self, key: &str, value: Value) -> Result<(), ConfigError> {
        let mut values = self.load_values()?;
        values.insert(key.to_string(), value);
        self.save_values(values)
    }

    fn load_values(&self) -> Result<HashMap<String, Value>, ConfigError> {
        if self.config_path.exists() {
            let file_content = fs::read_to_string(&self.config_path)?;
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
            let json_value: Value = serde_json::to_value(yaml_value)?;

            if let Value::Object(map) = json_value {
                return Ok(map.into_iter().collect());
            }
        }
        Ok(HashMap::new())
    }

    fn save_values(&self, values: HashMap<String, Value>) -> Result<(), ConfigError> {
        let yaml_value = serde_yaml::to_string(&values)?;
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::DirectoryError(e.to_string()))?;
        }
        fs::write(&self.config_path, yaml_value)?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), ConfigError> {
        let mut values = self.load_values()?;
        values.remove(key);
        self.save_values(values)
    }

    pub fn get_secret<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        self.get_secret_from_env(key).or_else(|| self.get_secret_from_keyring(key))
    }

    fn get_secret_from_env<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        self.get_from_env(key)
    }

    fn get_secret_from_keyring<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        let entry = Entry::new(&self.keyring_service, KEYRING_USERNAME)?;
        let content = entry.get_password().map_err(|e| ConfigError::KeyringError(e.to_string()))?;
        let values: HashMap<String, Value> = serde_json::from_str(&content)?;
        values.get(key).ok_or_else(|| ConfigError::NotFound(key.to_string())).and_then(|v| Ok(serde_json::from_value(v.clone())?))
    }

    pub fn set_secret(&self, key: &str, value: Value) -> Result<(), ConfigError> {
        let mut values = self.load_secrets()?;
        values.insert(key.to_string(), value);
        let json_value = serde_json::to_string(&values)?;
        let entry = Entry::new(&self.keyring_service, KEYRING_USERNAME)?;
        entry.set_password(&json_value)?;
        Ok(())
    }

    fn load_secrets(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let entry = Entry::new(&self.keyring_service, KEYRING_USERNAME)?;
        match entry.get_password() {
            Ok(content) => {
                let values: HashMap<String, Value> = serde_json::from_str(&content)?;
                Ok(values)
            },
            Err(keyring::Error::NoEntry) => Ok(HashMap::new()),
            Err(e) => Err(ConfigError::KeyringError(e.to_string())),
        }
    }

    pub fn delete_secret(&self, key: &str) -> Result<(), ConfigError> {
        let mut values = self.load_secrets()?;
        values.remove(key);
        let json_value = serde_json::to_string(&values)?;
        let entry = Entry::new(&self.keyring_service, KEYRING_USERNAME)?;
        entry.set_password(&json_value)?;
        Ok(())
    }
}
