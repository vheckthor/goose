use crate::bench_config::{BenchEval, BenchRunConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

/// Manages configuration and environment variables for the benchmark runner
pub struct ConfigManager {
    config: BenchRunConfig,
    env_vars: HashMap<String, String>,
}

impl ConfigManager {
    /// Create a new ConfigManager from a config string
    pub fn from_string(config_str: String) -> Result<Self> {
        let config = BenchRunConfig::from_string(config_str)?;
        let mut manager = Self {
            config,
            env_vars: HashMap::new(),
        };
        manager.load_environment_variables()?;
        Ok(manager)
    }

    /// Get a reference to the underlying configuration
    pub fn config(&self) -> &BenchRunConfig {
        &self.config
    }

    /// Load environment variables from the environment and the config's env_file
    fn load_environment_variables(&mut self) -> Result<()> {
        // Load from the actual environment
        for (key, value) in env::vars() {
            self.env_vars.insert(key, value);
        }

        // Load from env_file if specified
        if let Some(env_file) = &self.config.env_file {
            let file_vars = Self::parse_env_file(env_file)?;
            for (key, value) in file_vars {
                // Environment file variables override system environment
                self.env_vars.insert(key, value);
            }
        }

        Ok(())
    }

    /// Parse an environment file
    fn parse_env_file(path: &PathBuf) -> Result<Vec<(String, String)>> {
        let file = File::open(path).context(format!("Failed to open env file at {:?}", path))?;
        let reader = io::BufReader::new(file);
        let mut env_vars = Vec::new();

        for line in reader.lines() {
            let line = line?;
            // Skip empty lines and comments
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }

            // Split on first '=' only
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                // Remove quotes if present
                let value = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                env_vars.push((key, value));
            }
        }

        Ok(env_vars)
    }

    /// Get all environment variables as key-value pairs
    pub fn get_environment_variables(&self) -> Vec<(String, String)> {
        self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get toolshim-specific environment variables
    pub fn get_toolshim_environment(&self) -> Vec<(String, String)> {
        let mut shim_envs: Vec<(String, String)> = Vec::new();

        if let Some(model) = self.config.models.first() {
            if let Some(shim_opt) = &model.tool_shim {
                if shim_opt.use_tool_shim {
                    shim_envs.push(("GOOSE_TOOLSHIM".to_string(), "true".to_string()));
                    if let Some(shim_model) = &shim_opt.tool_shim_model {
                        shim_envs.push((
                            "GOOSE_TOOLSHIM_OLLAMA_MODEL".to_string(),
                            shim_model.clone(),
                        ));
                    }
                }
            }
        }

        shim_envs
    }

    /// Get specific environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    /// Create a new configuration with a subset of evals
    pub fn create_eval_config(&self, eval: &BenchEval, run_id: String) -> Result<String> {
        let mut new_config = self.config.clone();
        new_config.run_id = Some(run_id);
        new_config.evals = vec![eval.clone()];
        new_config.to_string()
    }
}
