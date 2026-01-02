//! LLM client factory for provider selection
//!
//! This module provides a factory for creating LLM clients based on
//! configuration or environment variables.

use std::sync::Arc;

use crate::env::{apis as env_apis, llm as env_llm};

use super::adapters::{ClaudeCodeClient, GeminiCliClient, GoogleAiAdapter};
use super::errors::LlmError;
use super::subprocess::check_cli_available;
use super::traits::LlmClient;
use super::types::{LlmConfig, LlmProvider};

/// Factory for creating LLM clients based on provider configuration
pub struct LlmClientFactory;

impl LlmClientFactory {
    /// Create an LLM client based on configuration
    pub fn create(config: LlmConfig) -> Result<Arc<dyn LlmClient>, LlmError> {
        match config.provider {
            LlmProvider::GoogleAi => {
                let adapter = GoogleAiAdapter::new(config)?;
                Ok(Arc::new(adapter))
            }
            LlmProvider::ClaudeCode => {
                let client = ClaudeCodeClient::new(config)?;
                Ok(Arc::new(client))
            }
            LlmProvider::GeminiCli => {
                let client = GeminiCliClient::new(config)?;
                Ok(Arc::new(client))
            }
        }
    }

    /// Create an LLM client from environment variables
    ///
    /// Environment variables checked:
    /// - RETROCHAT_LLM_PROVIDER: "google-ai" | "claude-code" | "gemini-cli"
    /// - RETROCHAT_LLM_MODEL: Model identifier (provider-specific)
    /// - GOOGLE_AI_API_KEY: API key for Google AI (if provider is google-ai)
    /// - CLAUDE_CODE_PATH: Custom path to Claude CLI binary
    /// - GEMINI_CLI_PATH: Custom path to Gemini CLI binary
    pub fn from_env() -> Result<Arc<dyn LlmClient>, LlmError> {
        let provider = std::env::var(env_llm::RETROCHAT_LLM_PROVIDER)
            .ok()
            .and_then(|s| s.parse::<LlmProvider>().ok())
            .unwrap_or(LlmProvider::GoogleAi);

        let mut config = LlmConfig {
            provider,
            model: std::env::var(env_llm::RETROCHAT_LLM_MODEL).ok(),
            timeout_secs: 300,
            max_retries: 3,
            api_key: None,
            cli_path: None,
        };

        // Set provider-specific configuration
        match provider {
            LlmProvider::GoogleAi => {
                config.api_key = std::env::var(env_apis::GOOGLE_AI_API_KEY).ok();
                if config.api_key.is_none() {
                    return Err(LlmError::ConfigurationError {
                        message: "GOOGLE_AI_API_KEY is required for google-ai provider".to_string(),
                    });
                }
            }
            LlmProvider::ClaudeCode => {
                config.cli_path = std::env::var(env_llm::CLAUDE_CODE_PATH).ok();
            }
            LlmProvider::GeminiCli => {
                config.cli_path = std::env::var(env_llm::GEMINI_CLI_PATH).ok();
            }
        }

        Self::create(config)
    }

    /// Create an LLM client with explicit provider
    ///
    /// For Google AI, an API key is required.
    /// For CLI providers, the binary path is optional (defaults to PATH lookup).
    pub fn for_provider(
        provider: LlmProvider,
        api_key: Option<String>,
    ) -> Result<Arc<dyn LlmClient>, LlmError> {
        let config = match provider {
            LlmProvider::GoogleAi => {
                let key = api_key.ok_or_else(|| LlmError::ConfigurationError {
                    message: "API key required for Google AI provider".to_string(),
                })?;
                LlmConfig::google_ai(key)
            }
            LlmProvider::ClaudeCode => LlmConfig::claude_code(),
            LlmProvider::GeminiCli => LlmConfig::gemini_cli(),
        };

        Self::create(config)
    }

    /// List available providers with their configuration status
    ///
    /// Returns a list of (provider, is_available, status_message) tuples.
    pub async fn list_available() -> Vec<(LlmProvider, bool, String)> {
        let mut result = Vec::new();

        // Check Google AI
        let google_available = std::env::var(env_apis::GOOGLE_AI_API_KEY).is_ok();
        result.push((
            LlmProvider::GoogleAi,
            google_available,
            if google_available {
                "Configured via GOOGLE_AI_API_KEY".to_string()
            } else {
                "Missing GOOGLE_AI_API_KEY".to_string()
            },
        ));

        // Check Claude Code CLI
        let claude_path =
            std::env::var(env_llm::CLAUDE_CODE_PATH).unwrap_or_else(|_| "claude".to_string());
        let claude_available = check_cli_available(&claude_path).await;
        result.push((
            LlmProvider::ClaudeCode,
            claude_available,
            if claude_available {
                format!("Found at: {claude_path}")
            } else {
                format!("Binary not found: {claude_path}")
            },
        ));

        // Check Gemini CLI
        let gemini_path =
            std::env::var(env_llm::GEMINI_CLI_PATH).unwrap_or_else(|_| "gemini".to_string());
        let gemini_available = check_cli_available(&gemini_path).await;
        result.push((
            LlmProvider::GeminiCli,
            gemini_available,
            if gemini_available {
                format!("Found at: {gemini_path}")
            } else {
                format!("Binary not found: {gemini_path}")
            },
        ));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_google_ai_without_key() {
        let config = LlmConfig::default(); // GoogleAi without api_key
        let result = LlmClientFactory::create(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_google_ai_with_key() {
        let config = LlmConfig::google_ai("test-key".to_string());
        let result = LlmClientFactory::create(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_claude_code() {
        let config = LlmConfig::claude_code();
        let result = LlmClientFactory::create(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_gemini_cli() {
        let config = LlmConfig::gemini_cli();
        let result = LlmClientFactory::create(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_provider_google_ai_requires_key() {
        let result = LlmClientFactory::for_provider(LlmProvider::GoogleAi, None);
        assert!(result.is_err());

        let result =
            LlmClientFactory::for_provider(LlmProvider::GoogleAi, Some("test-key".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_provider_cli_no_key_needed() {
        let result = LlmClientFactory::for_provider(LlmProvider::ClaudeCode, None);
        assert!(result.is_ok());

        let result = LlmClientFactory::for_provider(LlmProvider::GeminiCli, None);
        assert!(result.is_ok());
    }
}
