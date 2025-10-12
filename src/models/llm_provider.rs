use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LlmProvider {
    ClaudeCode,
    GeminiCLI,
    Codex,
    CursorAgent,
    Other(String),
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::ClaudeCode => write!(f, "Claude Code"),
            LlmProvider::GeminiCLI => write!(f, "Gemini CLI"),
            LlmProvider::Codex => write!(f, "Codex"),
            LlmProvider::CursorAgent => write!(f, "Cursor Agent"),
            LlmProvider::Other(name) => write!(f, "{name}"),
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Claude Code" => Ok(LlmProvider::ClaudeCode),
            "Gemini CLI" => Ok(LlmProvider::GeminiCLI),
            "Codex" => Ok(LlmProvider::Codex),
            "Cursor Agent" => Ok(LlmProvider::CursorAgent),
            _ => Ok(LlmProvider::Other(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParserType {
    ClaudeCodeJsonl,
    GeminiJson,
    CodexJson,
    CursorDb,
    Generic,
}

impl std::fmt::Display for ParserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserType::ClaudeCodeJsonl => write!(f, "claude-code-jsonl"),
            ParserType::GeminiJson => write!(f, "gemini-json"),
            ParserType::CodexJson => write!(f, "codex-json"),
            ParserType::CursorDb => write!(f, "cursor-db"),
            ParserType::Generic => write!(f, "generic"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
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

impl LlmProviderConfig {
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
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self::new("Unknown Provider".to_string(), ParserType::Generic)
    }
}

/// Create ClaudeCode provider configuration
pub fn create_claude_code_config() -> LlmProviderConfig {
    LlmProviderConfig::new("Claude Code".to_string(), ParserType::ClaudeCodeJsonl)
        .with_cli_name("claude".to_string())
        .with_description("Claude Code (.jsonl files)".to_string())
        .with_env_var_name("RETROCHAT_CLAUDE_DIRS".to_string())
        .with_default_directory("~/.claude/projects".to_string())
        .with_file_patterns(vec![
            "*.jsonl".to_string(),
            "*claude-code*.json*".to_string(),
        ])
        .with_default_location("darwin".to_string(), vec!["~/.claude/projects".to_string()])
        .with_default_location("linux".to_string(), vec!["~/.claude/projects".to_string()])
        .with_default_location(
            "windows".to_string(),
            vec!["%APPDATA%/Claude Code/".to_string()],
        )
        .with_token_support()
        .with_tool_support()
}

/// Create GeminiCLI provider configuration
pub fn create_gemini_cli_config() -> LlmProviderConfig {
    LlmProviderConfig::new("Gemini CLI".to_string(), ParserType::GeminiJson)
        .with_cli_name("gemini".to_string())
        .with_description("Gemini CLI (.json files)".to_string())
        .with_env_var_name("RETROCHAT_GEMINI_DIRS".to_string())
        .with_default_directory("~/.gemini/tmp".to_string())
        .with_file_patterns(vec!["*gemini*.json".to_string(), "*bard*.json".to_string()])
        .with_default_location("darwin".to_string(), vec!["~/.gemini/tmp".to_string()])
        .with_default_location("linux".to_string(), vec!["~/.gemini/tmp".to_string()])
        .with_default_location(
            "windows".to_string(),
            vec!["%APPDATA%/Gemini/tmp".to_string()],
        )
        .with_token_support()
}

/// Create Codex provider configuration
pub fn create_codex_config() -> LlmProviderConfig {
    LlmProviderConfig::new("Codex".to_string(), ParserType::CodexJson)
        .with_cli_name("codex".to_string())
        .with_description("Codex (various formats)".to_string())
        .with_env_var_name("RETROCHAT_CODEX_DIRS".to_string())
        .with_default_directory("~/.codex/sessions".to_string())
        .with_file_patterns(vec![
            "*codex*.json".to_string(),
            "*conversations*.json".to_string(),
        ])
        .with_default_location("darwin".to_string(), vec!["~/.codex/sessions".to_string()])
        .with_default_location("linux".to_string(), vec!["~/.codex/sessions".to_string()])
        .with_default_location(
            "windows".to_string(),
            vec!["%APPDATA%/Codex/sessions".to_string()],
        )
        .with_token_support()
        .with_tool_support()
}

/// Create CursorAgent provider configuration
pub fn create_cursor_agent_config() -> LlmProviderConfig {
    LlmProviderConfig::new("Cursor Agent".to_string(), ParserType::CursorDb)
        .with_cli_name("cursor-agent".to_string())
        .with_description("Cursor Agent (store.db files)".to_string())
        .with_env_var_name("RETROCHAT_CURSOR_DIRS".to_string())
        .with_default_directory("~/.cursor/chats".to_string())
        .with_file_patterns(vec!["store.db".to_string(), "*cursor*.db".to_string()])
        .with_default_location("darwin".to_string(), vec!["~/.cursor/chats".to_string()])
        .with_default_location("linux".to_string(), vec!["~/.cursor/chats".to_string()])
        .with_default_location(
            "windows".to_string(),
            vec!["%APPDATA%/Cursor/chats".to_string()],
        )
        .with_token_support()
        .with_tool_support()
}

pub struct LlmProviderRegistry {
    providers: HashMap<LlmProvider, LlmProviderConfig>,
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
        let claude_code = create_claude_code_config();
        let gemini = create_gemini_cli_config();
        let codex = create_codex_config();
        let cursor = create_cursor_agent_config();

        self.providers.insert(LlmProvider::ClaudeCode, claude_code);
        self.providers.insert(LlmProvider::GeminiCLI, gemini);
        self.providers.insert(LlmProvider::Codex, codex);
        self.providers.insert(LlmProvider::CursorAgent, cursor);
    }

    pub fn get_provider(&self, id: &LlmProvider) -> Option<&LlmProviderConfig> {
        self.providers.get(id)
    }

    pub fn detect_provider_from_file(&self, file_path: &str) -> Option<&LlmProviderConfig> {
        self.providers
            .values()
            .find(|provider| provider.matches_file(file_path))
    }

    pub fn add_provider(&mut self, id: LlmProvider, provider: LlmProviderConfig) {
        self.providers.insert(id, provider);
    }

    pub fn remove_provider(&mut self, id: &LlmProvider) {
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

    /// Get all known providers (returns configs for all registered providers)
    pub fn all_known(&self) -> Vec<&LlmProviderConfig> {
        let mut providers: Vec<_> = self.providers.iter().collect();
        // Sort by provider ID for consistent ordering
        providers.sort_by_key(|(id, _)| format!("{:?}", id));
        providers.into_iter().map(|(_, config)| config).collect()
    }

    /// Get CLI name for a provider
    pub fn cli_name(&self, id: &LlmProvider) -> Option<&str> {
        self.get_provider(id).map(|p| p.cli_name())
    }

    /// Get description for a provider
    pub fn description(&self, id: &LlmProvider) -> Option<&str> {
        self.get_provider(id).map(|p| p.description())
    }

    /// Get environment variable name for a provider
    pub fn env_var_name(&self, id: &LlmProvider) -> Option<&str> {
        self.get_provider(id).and_then(|p| p.env_var_name())
    }

    /// Get default directory for a provider
    pub fn default_directory(&self, id: &LlmProvider) -> Option<&str> {
        self.get_provider(id).and_then(|p| p.default_directory())
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
        let provider =
            LlmProviderConfig::new("Test Provider".to_string(), ParserType::ClaudeCodeJsonl);

        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.parser_type, ParserType::ClaudeCodeJsonl);
        assert!(!provider.supports_tokens);
        assert!(!provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_matching() {
        let provider = LlmProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
            .with_file_patterns(vec!["*.jsonl".to_string(), "claude".to_string()]);

        assert!(provider.matches_file("chat.jsonl"));
        assert!(provider.matches_file("claude-session.json")); // matches "claude" substring
        assert!(!provider.matches_file("data.txt"));
        assert!(!provider.matches_file("other.json")); // no matching pattern
    }

    #[test]
    fn test_provider_capabilities() {
        let mut provider = LlmProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl);

        provider.update_capabilities(true, true);
        assert!(provider.supports_tokens);
        assert!(provider.supports_tools);
    }

    #[test]
    fn test_file_pattern_management() {
        let mut provider = LlmProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl);

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
        assert!(registry.get_provider(&LlmProvider::ClaudeCode).is_some());
        assert!(registry.get_provider(&LlmProvider::GeminiCLI).is_some());
        assert!(registry.get_provider(&LlmProvider::Codex).is_some());
        assert!(registry.get_provider(&LlmProvider::CursorAgent).is_some());

        // Test detection
        let claude_provider = registry.detect_provider_from_file("chat.jsonl");
        assert!(claude_provider.is_some());
        assert_eq!(claude_provider.unwrap().name, "Claude Code");

        // Test Cursor detection
        let cursor_provider = registry.detect_provider_from_file("store.db");
        assert!(cursor_provider.is_some());
        assert_eq!(cursor_provider.unwrap().name, "Cursor Agent");
    }

    #[test]
    fn test_default_locations() {
        let provider = LlmProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
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
        assert_eq!(ParserType::CodexJson.to_string(), "codex-json");
        assert_eq!(ParserType::CursorDb.to_string(), "cursor-db");
        assert_eq!(ParserType::Generic.to_string(), "generic");
    }

    #[test]
    fn test_provider_config_delegate_methods() {
        let config = LlmProviderConfig::new("Test".to_string(), ParserType::ClaudeCodeJsonl)
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
    fn test_provider_config_delegate_methods_all_providers() {
        // Test that the create_*_config functions set the correct metadata
        let configs = vec![
            (
                create_claude_code_config(),
                "claude",
                "RETROCHAT_CLAUDE_DIRS",
            ),
            (
                create_cursor_agent_config(),
                "cursor-agent",
                "RETROCHAT_CURSOR_DIRS",
            ),
            (
                create_gemini_cli_config(),
                "gemini",
                "RETROCHAT_GEMINI_DIRS",
            ),
            (create_codex_config(), "codex", "RETROCHAT_CODEX_DIRS"),
        ];

        for (config, expected_cli_name, expected_env_var) in configs {
            assert_eq!(config.cli_name(), expected_cli_name);
            assert_eq!(config.env_var_name(), Some(expected_env_var));
            // Verify description contains the provider info
            assert!(!config.description().is_empty());
        }
    }

    #[test]
    fn test_create_claude_code_config() {
        let config = create_claude_code_config();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "claude");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.claude/projects"));

        // Check other properties
        assert_eq!(config.name, "Claude Code");
        assert_eq!(config.parser_type, ParserType::ClaudeCodeJsonl);
        assert!(config.supports_tokens);
        assert!(config.supports_tools);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_create_gemini_cli_config() {
        let config = create_gemini_cli_config();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "gemini");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.gemini/tmp"));

        // Check other properties
        assert_eq!(config.name, "Gemini CLI");
        assert_eq!(config.parser_type, ParserType::GeminiJson);
        assert!(config.supports_tokens);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_create_codex_config() {
        let config = create_codex_config();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "codex");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.codex/sessions"));

        // Check other properties
        assert_eq!(config.name, "Codex");
        assert_eq!(config.parser_type, ParserType::CodexJson);
        assert!(config.supports_tokens);
        assert!(config.supports_tools);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_create_cursor_agent_config() {
        let config = create_cursor_agent_config();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "cursor-agent");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.cursor/chats"));

        // Check other properties
        assert_eq!(config.name, "Cursor Agent");
        assert_eq!(config.parser_type, ParserType::CursorDb);
        assert!(config.supports_tokens);
        assert!(config.supports_tools);
        assert!(!config.file_patterns.is_empty());
    }
}
