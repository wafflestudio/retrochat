use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::provider::ParserType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub cli_name: String,
    pub description: String,
    pub env_var_name: Option<String>,
    pub default_directory: Option<String>,
    pub file_patterns: Vec<String>,
    pub default_locations: HashMap<String, Vec<String>>, // OS -> paths
    pub parser_type: ParserType,
    pub supports_tokens: bool,
    pub supports_tools: bool,
    pub last_updated: DateTime<Utc>,
}

impl ProviderConfig {
    pub fn new(name: String, parser_type: ParserType) -> Self {
        Self {
            name: name.clone(),
            cli_name: name.clone(),
            description: format!("{} provider", name),
            env_var_name: None,
            default_directory: None,
            file_patterns: Vec::new(),
            default_locations: HashMap::new(),
            parser_type,
            supports_tokens: false,
            supports_tools: false,
            last_updated: Utc::now(),
        }
    }

    pub fn with_cli_name(mut self, cli_name: String) -> Self {
        self.cli_name = cli_name;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_env_var_name(mut self, env_var_name: String) -> Self {
        self.env_var_name = Some(env_var_name);
        self
    }

    pub fn with_default_directory(mut self, default_directory: String) -> Self {
        self.default_directory = Some(default_directory);
        self
    }

    pub fn with_file_patterns(mut self, patterns: Vec<String>) -> Self {
        self.file_patterns = patterns;
        self
    }

    pub fn with_default_location(mut self, os: String, paths: Vec<String>) -> Self {
        self.default_locations.insert(os, paths);
        self
    }

    pub fn with_token_support(mut self) -> Self {
        self.supports_tokens = true;
        self
    }

    pub fn with_tool_support(mut self) -> Self {
        self.supports_tools = true;
        self
    }

    pub fn matches_file(&self, file_path: &str) -> bool {
        if self.file_patterns.is_empty() {
            return false;
        }

        self.file_patterns.iter().any(|pattern| {
            // Simple glob pattern matching
            if pattern.contains('*') {
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    file_path.starts_with(parts[0]) && file_path.ends_with(parts[1])
                } else {
                    false
                }
            } else {
                file_path.contains(pattern)
            }
        })
    }

    pub fn get_default_locations_for_os(&self, os: &str) -> Vec<String> {
        self.default_locations.get(os).cloned().unwrap_or_default()
    }

    pub fn add_file_pattern(&mut self, pattern: String) {
        if !self.file_patterns.contains(&pattern) {
            self.file_patterns.push(pattern);
            self.last_updated = Utc::now();
        }
    }

    pub fn remove_file_pattern(&mut self, pattern: &str) {
        if let Some(pos) = self.file_patterns.iter().position(|p| p == pattern) {
            self.file_patterns.remove(pos);
            self.last_updated = Utc::now();
        }
    }

    pub fn update_capabilities(&mut self, supports_tokens: bool, supports_tools: bool) {
        self.supports_tokens = supports_tokens;
        self.supports_tools = supports_tools;
        self.last_updated = Utc::now();
    }

    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && !self.file_patterns.is_empty()
    }

    /// Get CLI name for this provider
    pub fn cli_name(&self) -> &str {
        &self.cli_name
    }

    /// Get description for this provider
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get environment variable name for this provider
    pub fn env_var_name(&self) -> Option<&str> {
        self.env_var_name.as_deref()
    }

    /// Get default directory for this provider
    pub fn default_directory(&self) -> Option<&str> {
        self.default_directory.as_deref()
    }

    /// Get directories to import from based on environment variable or default
    pub fn get_import_directories(&self) -> Vec<String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

        // Get directories from environment variable or default
        let dirs_str = if let Some(env_var) = &self.env_var_name {
            std::env::var(env_var).unwrap_or_else(|_| {
                self.default_directory
                    .as_ref()
                    .map(|d| d.replace('~', &home))
                    .unwrap_or_default()
            })
        } else {
            self.default_directory
                .as_ref()
                .map(|d| d.replace('~', &home))
                .unwrap_or_default()
        };

        // Split by colon and expand tildes
        dirs_str
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|dir_str| {
                if dir_str.starts_with('~') {
                    dir_str.replacen('~', &home, 1)
                } else {
                    dir_str.to_string()
                }
            })
            .collect()
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::new("Unknown Provider".to_string(), ParserType::Generic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider_config() {
        let provider =
            ProviderConfig::new("Test Provider".to_string(), ParserType::ClaudeCodeJsonl);

        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.parser_type, ParserType::ClaudeCodeJsonl);
        assert!(!provider.supports_tokens);
        assert!(!provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_matching() {
        let provider = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_file_patterns(vec!["*.jsonl".to_string(), "claude".to_string()]);

        assert!(provider.matches_file("chat.jsonl"));
        assert!(provider.matches_file("claude-session.json")); // matches "claude" substring
        assert!(!provider.matches_file("data.txt"));
        assert!(!provider.matches_file("other.json")); // no matching pattern
    }

    #[test]
    fn test_provider_capabilities() {
        let mut provider = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl);

        provider.update_capabilities(true, true);
        assert!(provider.supports_tokens);
        assert!(provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_management() {
        let mut provider = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl);

        provider.add_file_pattern("*.jsonl".to_string());
        assert_eq!(provider.file_patterns.len(), 1);

        provider.add_file_pattern("*.jsonl".to_string()); // Should not duplicate
        assert_eq!(provider.file_patterns.len(), 1);

        provider.add_file_pattern("*.json".to_string());
        assert_eq!(provider.file_patterns.len(), 2);

        provider.remove_file_pattern("*.jsonl");
        assert_eq!(provider.file_patterns.len(), 1);
        assert_eq!(provider.file_patterns[0], "*.json");
    }

    #[test]
    fn test_default_locations() {
        let provider = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_default_location(
                "darwin".to_string(),
                vec!["/Users/test/Library/".to_string()],
            );

        let locations = provider.get_default_locations_for_os("darwin");
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0], "/Users/test/Library/");

        let empty_locations = provider.get_default_locations_for_os("windows");
        assert!(empty_locations.is_empty());
    }

    #[test]
    fn test_provider_config_delegate_methods() {
        let config = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_cli_name("test-cli".to_string())
            .with_description("Test description".to_string())
            .with_env_var_name("TEST_ENV_VAR".to_string())
            .with_default_directory("/test/path".to_string());

        // Test that config methods return the correct values
        assert_eq!(config.cli_name(), "test-cli");
        assert_eq!(config.description(), "Test description");
        assert_eq!(config.env_var_name(), Some("TEST_ENV_VAR"));
        assert_eq!(config.default_directory(), Some("/test/path"));
    }

    #[test]
    fn test_get_import_directories_with_default() {
        let config = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_default_directory("~/.test/dir".to_string());

        let dirs = config.get_import_directories();
        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with("/.test/dir"));
    }

    #[test]
    fn test_get_import_directories_multiple() {
        std::env::set_var("TEST_MULTI_DIRS", "/path1:/path2:~/.path3");
        let config = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_env_var_name("TEST_MULTI_DIRS".to_string())
            .with_default_directory("~/.test/default".to_string());

        let dirs = config.get_import_directories();
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0], "/path1");
        assert_eq!(dirs[1], "/path2");
        assert!(dirs[2].ends_with("/.path3"));

        std::env::remove_var("TEST_MULTI_DIRS");
    }

    #[test]
    fn test_get_import_directories_empty() {
        let config = ProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl);

        let dirs = config.get_import_directories();
        assert_eq!(dirs.len(), 0);
    }
}
