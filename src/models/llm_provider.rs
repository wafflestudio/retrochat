use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::chat_session::LlmProvider as LlmProviderEnum;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParserType {
    ClaudeCodeJsonl,
    GeminiJson,
    ChatGptJson,
    Generic,
}

impl std::fmt::Display for ParserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserType::ClaudeCodeJsonl => write!(f, "claude-code-jsonl"),
            ParserType::GeminiJson => write!(f, "gemini-json"),
            ParserType::ChatGptJson => write!(f, "chatgpt-json"),
            ParserType::Generic => write!(f, "generic"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: LlmProviderEnum,
    pub name: String,
    pub file_patterns: Vec<String>,
    pub default_locations: HashMap<String, Vec<String>>, // OS -> paths
    pub parser_type: ParserType,
    pub supports_tokens: bool,
    pub supports_tools: bool,
    pub last_updated: DateTime<Utc>,
}

impl LlmProviderConfig {
    pub fn new(id: LlmProviderEnum, name: String, parser_type: ParserType) -> Self {
        Self {
            id,
            name,
            file_patterns: Vec::new(),
            default_locations: HashMap::new(),
            parser_type,
            supports_tokens: false,
            supports_tools: false,
            last_updated: Utc::now(),
        }
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
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self::new(
            LlmProviderEnum::Other("unknown".to_string()),
            "Unknown Provider".to_string(),
            ParserType::Generic,
        )
    }
}

pub struct LlmProviderRegistry {
    providers: HashMap<LlmProviderEnum, LlmProviderConfig>,
}

impl LlmProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
        };
        registry.load_default_providers();
        registry
    }

    pub fn load_default_providers(&mut self) {
        // Claude Code provider
        let claude_code = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Claude Code".to_string(),
            ParserType::ClaudeCodeJsonl,
        )
        .with_file_patterns(vec![
            "*.jsonl".to_string(),
            "*claude-code*.json*".to_string(),
        ])
        .with_default_location(
            "darwin".to_string(),
            vec!["~/Library/Application Support/Claude Code/".to_string()],
        )
        .with_default_location(
            "linux".to_string(),
            vec!["~/.config/claude-code/".to_string()],
        )
        .with_default_location(
            "windows".to_string(),
            vec!["%APPDATA%/Claude Code/".to_string()],
        )
        .with_token_support()
        .with_tool_support();

        // Gemini provider
        let gemini = LlmProviderConfig::new(
            LlmProviderEnum::Gemini,
            "Google Gemini".to_string(),
            ParserType::GeminiJson,
        )
        .with_file_patterns(vec!["*gemini*.json".to_string(), "*bard*.json".to_string()])
        .with_token_support();

        // ChatGPT provider
        let chatgpt = LlmProviderConfig::new(
            LlmProviderEnum::ChatGpt,
            "ChatGPT".to_string(),
            ParserType::ChatGptJson,
        )
        .with_file_patterns(vec![
            "*chatgpt*.json".to_string(),
            "*conversations*.json".to_string(),
        ])
        .with_token_support()
        .with_tool_support();

        self.providers
            .insert(LlmProviderEnum::ClaudeCode, claude_code);
        self.providers.insert(LlmProviderEnum::Gemini, gemini);
        self.providers.insert(LlmProviderEnum::ChatGpt, chatgpt);
    }

    pub fn get_provider(&self, id: &LlmProviderEnum) -> Option<&LlmProviderConfig> {
        self.providers.get(id)
    }

    pub fn detect_provider_from_file(&self, file_path: &str) -> Option<&LlmProviderConfig> {
        self.providers
            .values()
            .find(|provider| provider.matches_file(file_path))
    }

    pub fn add_provider(&mut self, provider: LlmProviderConfig) {
        self.providers.insert(provider.id.clone(), provider);
    }

    pub fn remove_provider(&mut self, id: &LlmProviderEnum) {
        self.providers.remove(id);
    }

    pub fn list_providers(&self) -> Vec<&LlmProviderConfig> {
        self.providers.values().collect()
    }

    pub fn get_supported_patterns(&self) -> Vec<String> {
        self.providers
            .values()
            .flat_map(|provider| provider.file_patterns.iter())
            .cloned()
            .collect()
    }
}

impl Default for LlmProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider_config() {
        let provider = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Test Provider".to_string(),
            ParserType::ClaudeCodeJsonl,
        );

        assert_eq!(provider.id, LlmProviderEnum::ClaudeCode);
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.parser_type, ParserType::ClaudeCodeJsonl);
        assert!(!provider.supports_tokens);
        assert!(!provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_matching() {
        let provider = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Test".to_string(),
            ParserType::ClaudeCodeJsonl,
        )
        .with_file_patterns(vec!["*.jsonl".to_string(), "claude".to_string()]);

        assert!(provider.matches_file("chat.jsonl"));
        assert!(provider.matches_file("claude-session.json")); // matches "claude" substring
        assert!(!provider.matches_file("data.txt"));
        assert!(!provider.matches_file("other.json")); // no matching pattern
    }

    #[test]
    fn test_provider_capabilities() {
        let mut provider = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Test".to_string(),
            ParserType::ClaudeCodeJsonl,
        );

        provider.update_capabilities(true, true);
        assert!(provider.supports_tokens);
        assert!(provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_management() {
        let mut provider = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Test".to_string(),
            ParserType::ClaudeCodeJsonl,
        );

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
    fn test_provider_registry() {
        let registry = LlmProviderRegistry::new();

        // Should have default providers
        assert!(registry
            .get_provider(&LlmProviderEnum::ClaudeCode)
            .is_some());
        assert!(registry.get_provider(&LlmProviderEnum::Gemini).is_some());
        assert!(registry.get_provider(&LlmProviderEnum::ChatGpt).is_some());

        // Test detection
        let claude_provider = registry.detect_provider_from_file("chat.jsonl");
        assert!(claude_provider.is_some());
        assert_eq!(claude_provider.unwrap().id, LlmProviderEnum::ClaudeCode);
    }

    #[test]
    fn test_default_locations() {
        let provider = LlmProviderConfig::new(
            LlmProviderEnum::ClaudeCode,
            "Test".to_string(),
            ParserType::ClaudeCodeJsonl,
        )
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
    fn test_parser_type_display() {
        assert_eq!(ParserType::ClaudeCodeJsonl.to_string(), "claude-code-jsonl");
        assert_eq!(ParserType::GeminiJson.to_string(), "gemini-json");
        assert_eq!(ParserType::ChatGptJson.to_string(), "chatgpt-json");
        assert_eq!(ParserType::Generic.to_string(), "generic");
    }
}
