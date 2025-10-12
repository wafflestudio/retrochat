use std::collections::HashMap;

use super::config::{
    ClaudeCodeConfig, CodexConfig, CursorAgentConfig, GeminiCliConfig, ProviderConfig,
};
use super::r#enum::Provider;

pub struct ProviderRegistry {
    providers: HashMap<Provider, ProviderConfig>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
        };
        registry.load_default_providers();
        registry
    }

    /// List providers supported via CLI (excluding `All` aggregate)
    pub fn supported_providers() -> Vec<Provider> {
        Provider::all_concrete()
    }

    pub fn load_default_providers(&mut self) {
        // Get all supported providers and load their configurations
        for provider in Self::supported_providers() {
            let config = match provider {
                Provider::ClaudeCode => ClaudeCodeConfig::create(),
                Provider::GeminiCLI => GeminiCliConfig::create(),
                Provider::Codex => CodexConfig::create(),
                Provider::CursorAgent => CursorAgentConfig::create(),
                Provider::All => continue, // Skip aggregate
                Provider::Other(_) => continue, // Skip unknown providers
            };
            self.providers.insert(provider, config);
        }
    }

    pub fn get_provider(&self, id: &Provider) -> Option<&ProviderConfig> {
        self.providers.get(id)
    }

    pub fn detect_provider_from_file(&self, file_path: &str) -> Option<&ProviderConfig> {
        self.providers
            .values()
            .find(|provider| provider.matches_file(file_path))
    }

    pub fn add_provider(&mut self, id: Provider, provider: ProviderConfig) {
        self.providers.insert(id, provider);
    }

    pub fn remove_provider(&mut self, id: &Provider) {
        self.providers.remove(id);
    }

    pub fn list_providers(&self) -> Vec<&ProviderConfig> {
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
    pub fn all_known(&self) -> Vec<&ProviderConfig> {
        let mut providers: Vec<_> = self.providers.iter().collect();
        // Sort by provider ID for consistent ordering
        providers.sort_by_key(|(id, _)| format!("{id:?}"));
        providers.into_iter().map(|(_, config)| config).collect()
    }

    /// Get CLI name for a provider
    pub fn cli_name(&self, id: &Provider) -> Option<&str> {
        self.get_provider(id).map(|p| p.cli_name())
    }

    /// Get description for a provider
    pub fn description(&self, id: &Provider) -> Option<&str> {
        self.get_provider(id).map(|p| p.description())
    }

    /// Get environment variable name for a provider
    pub fn env_var_name(&self, id: &Provider) -> Option<&str> {
        self.get_provider(id).and_then(|p| p.env_var_name())
    }

    /// Get default directory for a provider
    pub fn default_directory(&self, id: &Provider) -> Option<&str> {
        self.get_provider(id).and_then(|p| p.default_directory())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let registry = ProviderRegistry::new();

        // Should have default providers
        assert!(registry.get_provider(&Provider::ClaudeCode).is_some());
        assert!(registry.get_provider(&Provider::GeminiCLI).is_some());
        assert!(registry.get_provider(&Provider::Codex).is_some());
        assert!(registry.get_provider(&Provider::CursorAgent).is_some());

        // Test detection
        let claude_provider = registry.detect_provider_from_file("chat.jsonl");
        assert!(claude_provider.is_some());
        assert_eq!(claude_provider.unwrap().name, "Claude Code");

        // Test Cursor detection
        let cursor_provider = registry.detect_provider_from_file("store.db");
        assert!(cursor_provider.is_some());
        assert_eq!(cursor_provider.unwrap().name, "Cursor Agent");
    }
}
