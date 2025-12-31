//! LLM Provider factory
//!
//! This module provides functionality to create LLM providers based on
//! configuration or environment variables.

use std::sync::Arc;

use super::claude_code::{ClaudeCodeConfig, ClaudeCodeProvider};
use super::gemini_cli::{GeminiCliConfig, GeminiCliProvider};
use super::google_ai::GoogleAiProvider;
use super::{LlmError, LlmProvider, LlmProviderType};
use crate::env::llm as llm_env;
use crate::services::google_ai::GoogleAiConfig;

/// Configuration for LLM provider creation
#[derive(Debug, Clone, Default)]
pub struct LlmProviderConfig {
    /// The type of provider to create
    pub provider_type: LlmProviderType,
    /// Optional API key for Google AI (overrides environment variable)
    pub google_api_key: Option<String>,
    /// Optional path to Claude CLI binary
    pub claude_binary_path: Option<String>,
    /// Optional path to Gemini CLI binary
    pub gemini_binary_path: Option<String>,
    /// Optional model name override
    pub model: Option<String>,
}

impl LlmProviderConfig {
    /// Create a new configuration for a specific provider type
    pub fn new(provider_type: LlmProviderType) -> Self {
        Self {
            provider_type,
            ..Default::default()
        }
    }

    /// Set the Google AI API key
    pub fn with_google_api_key(mut self, key: impl Into<String>) -> Self {
        self.google_api_key = Some(key.into());
        self
    }

    /// Set the Claude CLI binary path
    pub fn with_claude_binary_path(mut self, path: impl Into<String>) -> Self {
        self.claude_binary_path = Some(path.into());
        self
    }

    /// Set the Gemini CLI binary path
    pub fn with_gemini_binary_path(mut self, path: impl Into<String>) -> Self {
        self.gemini_binary_path = Some(path.into());
        self
    }

    /// Set a model name override
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        let provider_type = std::env::var(llm_env::LLM_PROVIDER)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let google_api_key = std::env::var(crate::env::apis::GOOGLE_AI_API_KEY).ok();
        let claude_binary_path = std::env::var(llm_env::CLAUDE_BINARY_PATH).ok();
        let gemini_binary_path = std::env::var(llm_env::GEMINI_BINARY_PATH).ok();
        let model = std::env::var(llm_env::LLM_MODEL).ok();

        Self {
            provider_type,
            google_api_key,
            claude_binary_path,
            gemini_binary_path,
            model,
        }
    }
}

/// Create an LLM provider based on the given configuration
pub fn create_provider(config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>, LlmError> {
    match config.provider_type {
        LlmProviderType::GoogleAi => create_google_ai_provider(&config),
        LlmProviderType::ClaudeCode => create_claude_code_provider(&config),
        LlmProviderType::GeminiCli => create_gemini_cli_provider(&config),
    }
}

/// Create a Google AI provider from the configuration
fn create_google_ai_provider(config: &LlmProviderConfig) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let api_key = config
        .google_api_key
        .clone()
        .or_else(|| std::env::var(crate::env::apis::GOOGLE_AI_API_KEY).ok())
        .ok_or_else(|| LlmError::Configuration {
            message: format!(
                "Google AI API key is required. Set {} environment variable.",
                crate::env::apis::GOOGLE_AI_API_KEY
            ),
        })?;

    let mut google_config = GoogleAiConfig::new(api_key);

    if let Some(model) = &config.model {
        google_config = google_config.with_model(model.clone());
    }

    let provider = GoogleAiProvider::new(google_config)?;
    Ok(Arc::new(provider))
}

/// Create a Claude Code CLI provider from the configuration
fn create_claude_code_provider(
    config: &LlmProviderConfig,
) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let mut claude_config = ClaudeCodeConfig::default();

    if let Some(binary_path) = &config.claude_binary_path {
        claude_config = claude_config.with_binary_path(binary_path.clone());
    } else if let Ok(path) = std::env::var(llm_env::CLAUDE_BINARY_PATH) {
        claude_config = claude_config.with_binary_path(path);
    }

    if let Some(model) = &config.model {
        claude_config = claude_config.with_model(model.clone());
    }

    let provider = ClaudeCodeProvider::with_config(claude_config);
    Ok(Arc::new(provider))
}

/// Create a Gemini CLI provider from the configuration
fn create_gemini_cli_provider(
    config: &LlmProviderConfig,
) -> Result<Arc<dyn LlmProvider>, LlmError> {
    let mut gemini_config = GeminiCliConfig::default();

    if let Some(binary_path) = &config.gemini_binary_path {
        gemini_config = gemini_config.with_binary_path(binary_path.clone());
    } else if let Ok(path) = std::env::var(llm_env::GEMINI_BINARY_PATH) {
        gemini_config = gemini_config.with_binary_path(path);
    }

    if let Some(model) = &config.model {
        gemini_config = gemini_config.with_model(model.clone());
    }

    let provider = GeminiCliProvider::with_config(gemini_config);
    Ok(Arc::new(provider))
}

/// Auto-detect and create the best available provider
///
/// This function tries providers in the following order:
/// 1. If RETROCHAT_LLM_PROVIDER is set, use that provider
/// 2. If GOOGLE_AI_API_KEY is set, use Google AI
/// 3. If Claude CLI is available, use Claude Code
/// 4. If Gemini CLI is available, use Gemini CLI
/// 5. Return an error if no provider is available
pub async fn auto_detect_provider() -> Result<Arc<dyn LlmProvider>, LlmError> {
    // First, check if a specific provider is requested via environment variable
    if let Ok(provider_str) = std::env::var(llm_env::LLM_PROVIDER) {
        if provider_str.parse::<LlmProviderType>().is_ok() {
            let config = LlmProviderConfig::from_env();
            return create_provider(config);
        }
    }

    // Try Google AI if API key is available
    if std::env::var(crate::env::apis::GOOGLE_AI_API_KEY).is_ok() {
        let config = LlmProviderConfig::new(LlmProviderType::GoogleAi);
        if let Ok(provider) = create_provider(config) {
            return Ok(provider);
        }
    }

    // Try Claude Code CLI
    let claude_provider = ClaudeCodeProvider::new();
    if claude_provider.is_available().await {
        return Ok(Arc::new(claude_provider));
    }

    // Try Gemini CLI
    let gemini_provider = GeminiCliProvider::new();
    if gemini_provider.is_available().await {
        return Ok(Arc::new(gemini_provider));
    }

    Err(LlmError::NotAvailable {
        message: "No LLM provider is available. Please either:\n\
            - Set GOOGLE_AI_API_KEY for Google AI API\n\
            - Install and authenticate Claude Code CLI\n\
            - Install and authenticate Gemini CLI"
            .to_string(),
    })
}

/// Get a list of all available providers on the system
pub async fn list_available_providers() -> Vec<(LlmProviderType, bool, String)> {
    let mut providers = Vec::new();

    // Check Google AI
    let google_available = std::env::var(crate::env::apis::GOOGLE_AI_API_KEY).is_ok();
    let google_reason = if google_available {
        "API key configured".to_string()
    } else {
        "GOOGLE_AI_API_KEY not set".to_string()
    };
    providers.push((LlmProviderType::GoogleAi, google_available, google_reason));

    // Check Claude Code
    let claude_provider = ClaudeCodeProvider::new();
    let claude_available = claude_provider.is_available().await;
    let claude_reason = if claude_available {
        "CLI binary found".to_string()
    } else {
        "claude CLI not found in PATH".to_string()
    };
    providers.push((LlmProviderType::ClaudeCode, claude_available, claude_reason));

    // Check Gemini CLI
    let gemini_provider = GeminiCliProvider::new();
    let gemini_available = gemini_provider.is_available().await;
    let gemini_reason = if gemini_available {
        "CLI binary found".to_string()
    } else {
        "gemini CLI not found in PATH".to_string()
    };
    providers.push((LlmProviderType::GeminiCli, gemini_available, gemini_reason));

    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = LlmProviderConfig::default();
        assert_eq!(config.provider_type, LlmProviderType::GoogleAi);
        assert!(config.google_api_key.is_none());
        assert!(config.claude_binary_path.is_none());
        assert!(config.gemini_binary_path.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = LlmProviderConfig::new(LlmProviderType::ClaudeCode)
            .with_claude_binary_path("/usr/local/bin/claude")
            .with_model("claude-3-opus");

        assert_eq!(config.provider_type, LlmProviderType::ClaudeCode);
        assert_eq!(
            config.claude_binary_path,
            Some("/usr/local/bin/claude".to_string())
        );
        assert_eq!(config.model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_create_google_ai_provider_with_key() {
        let config =
            LlmProviderConfig::new(LlmProviderType::GoogleAi).with_google_api_key("test-key");

        let provider = create_provider(config);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_type(), LlmProviderType::GoogleAi);
    }

    #[test]
    fn test_create_google_ai_provider_without_key() {
        // Temporarily unset the environment variable
        let original = std::env::var(crate::env::apis::GOOGLE_AI_API_KEY).ok();
        std::env::remove_var(crate::env::apis::GOOGLE_AI_API_KEY);

        let config = LlmProviderConfig::new(LlmProviderType::GoogleAi);
        let provider = create_provider(config);
        assert!(provider.is_err());

        // Restore original value
        if let Some(val) = original {
            std::env::set_var(crate::env::apis::GOOGLE_AI_API_KEY, val);
        }
    }

    #[test]
    fn test_create_claude_code_provider() {
        let config = LlmProviderConfig::new(LlmProviderType::ClaudeCode);
        let provider = create_provider(config);
        assert!(provider.is_ok());
        assert_eq!(
            provider.unwrap().provider_type(),
            LlmProviderType::ClaudeCode
        );
    }

    #[test]
    fn test_create_gemini_cli_provider() {
        let config = LlmProviderConfig::new(LlmProviderType::GeminiCli);
        let provider = create_provider(config);
        assert!(provider.is_ok());
        assert_eq!(
            provider.unwrap().provider_type(),
            LlmProviderType::GeminiCli
        );
    }

    #[tokio::test]
    async fn test_list_available_providers() {
        let providers = list_available_providers().await;
        assert_eq!(providers.len(), 3);

        // Check that all provider types are present
        let types: Vec<_> = providers.iter().map(|(t, _, _)| *t).collect();
        assert!(types.contains(&LlmProviderType::GoogleAi));
        assert!(types.contains(&LlmProviderType::ClaudeCode));
        assert!(types.contains(&LlmProviderType::GeminiCli));
    }
}
