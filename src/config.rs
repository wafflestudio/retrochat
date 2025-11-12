//! Configuration file management for RetroChat
//!
//! This module handles reading and writing configuration values to ~/.retrochat/config.toml
//! Configuration values can be overridden by environment variables.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::env::apis as env_apis;

/// Configuration structure matching config.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_ai_api_key: Option<String>,
}

impl Config {
    /// Get the config file path (~/.retrochat/config.toml)
    pub fn get_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        Ok(home_dir.join(".retrochat").join("config.toml"))
    }

    /// Load configuration from file
    /// Returns default config if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        // Set file permissions to 600 (owner read/write only) for security
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&config_path, permissions).with_context(|| {
                format!(
                    "Failed to set permissions on config file: {}",
                    config_path.display()
                )
            })?;
        }

        Ok(())
    }

    /// Get a config value by key
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "google-ai-api-key" | "google_ai_api_key" => self.api.google_ai_api_key.clone(),
            _ => None,
        }
    }

    /// Set a config value by key
    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        match key {
            "google-ai-api-key" | "google_ai_api_key" => {
                self.api.google_ai_api_key = Some(value);
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }

    /// Unset (remove) a config value by key
    pub fn unset(&mut self, key: &str) -> Result<()> {
        match key {
            "google-ai-api-key" | "google_ai_api_key" => {
                self.api.google_ai_api_key = None;
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }

    /// Get all config values as key-value pairs
    pub fn list(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();

        if let Some(ref key) = self.api.google_ai_api_key {
            items.push(("google-ai-api-key".to_string(), mask_api_key(key)));
        }

        items
    }
}

/// Get Google AI API key with priority: environment variable > config file
pub fn get_google_ai_api_key() -> Result<Option<String>> {
    // Priority 1: Environment variable
    if let Ok(key) = std::env::var(env_apis::GOOGLE_AI_API_KEY) {
        if !key.is_empty() {
            return Ok(Some(key));
        }
    }

    // Priority 2: Config file
    let config = Config::load()?;
    Ok(config.api.google_ai_api_key)
}

/// Check if Google AI API key is configured (either in env or config file)
pub fn has_google_ai_api_key() -> bool {
    get_google_ai_api_key().ok().flatten().is_some()
}

/// Mask API key for display (show first 4 and last 4 characters)
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("short"), "*****");
        assert_eq!(mask_api_key("12345678"), "********");
        assert_eq!(mask_api_key("1234567890abcdef"), "1234...cdef");
    }

    #[test]
    fn test_config_set_get() {
        let mut config = Config::default();

        config
            .set("google-ai-api-key", "test-key".to_string())
            .unwrap();
        assert_eq!(
            config.get("google-ai-api-key"),
            Some("test-key".to_string())
        );

        config.unset("google-ai-api-key").unwrap();
        assert_eq!(config.get("google-ai-api-key"), None);
    }
}
