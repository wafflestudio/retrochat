//! LLM types for multi-provider support
//!
//! This module provides provider-agnostic types for LLM interactions.

use serde::{Deserialize, Serialize};

/// LLM provider enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProvider {
    /// Google AI API (Gemini models via REST API)
    #[default]
    GoogleAi,
    /// Claude Code CLI (local subprocess)
    ClaudeCode,
    /// Gemini CLI (local subprocess)
    GeminiCli,
}

impl std::str::FromStr for LlmProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "google" | "google-ai" | "googleai" | "gemini-api" => Ok(LlmProvider::GoogleAi),
            "claude" | "claude-code" | "claudecode" => Ok(LlmProvider::ClaudeCode),
            "gemini" | "gemini-cli" | "geminicli" => Ok(LlmProvider::GeminiCli),
            _ => Err(format!(
                "Unknown LLM provider: {s}. Valid options: google-ai, claude-code, gemini-cli"
            )),
        }
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::GoogleAi => write!(f, "google-ai"),
            LlmProvider::ClaudeCode => write!(f, "claude-code"),
            LlmProvider::GeminiCli => write!(f, "gemini-cli"),
        }
    }
}

/// Request for text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// The prompt text to send to the LLM
    pub prompt: String,

    /// Maximum tokens to generate (optional, provider defaults apply)
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 - 1.0, optional)
    pub temperature: Option<f32>,

    /// System instruction or context (optional, not all providers support)
    pub system_prompt: Option<String>,
}

impl GenerateRequest {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_system_prompt(mut self, system_prompt: String) -> Self {
        self.system_prompt = Some(system_prompt);
        self
    }
}

/// Response from text generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateResponse {
    /// The generated text content
    pub text: String,

    /// Token usage (if reported by provider)
    pub token_usage: Option<TokenUsage>,

    /// Model used for generation
    pub model_used: Option<String>,

    /// Reason for stopping generation
    pub finish_reason: Option<String>,

    /// Provider-specific metadata (JSON value)
    pub metadata: Option<serde_json::Value>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Configuration for LLM client creation
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub model: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: usize,

    /// API key for remote providers (Google AI)
    pub api_key: Option<String>,

    /// Custom CLI binary path for subprocess providers
    pub cli_path: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::GoogleAi,
            model: None,
            timeout_secs: 300,
            max_retries: 3,
            api_key: None,
            cli_path: None,
        }
    }
}

impl LlmConfig {
    /// Create config for Google AI provider
    pub fn google_ai(api_key: String) -> Self {
        Self {
            provider: LlmProvider::GoogleAi,
            api_key: Some(api_key),
            ..Default::default()
        }
    }

    /// Create config for Claude Code CLI provider
    pub fn claude_code() -> Self {
        Self {
            provider: LlmProvider::ClaudeCode,
            ..Default::default()
        }
    }

    /// Create config for Gemini CLI provider
    pub fn gemini_cli() -> Self {
        Self {
            provider: LlmProvider::GeminiCli,
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_cli_path(mut self, path: String) -> Self {
        self.cli_path = Some(path);
        self
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_from_str() {
        assert_eq!(
            "google-ai".parse::<LlmProvider>().unwrap(),
            LlmProvider::GoogleAi
        );
        assert_eq!(
            "google".parse::<LlmProvider>().unwrap(),
            LlmProvider::GoogleAi
        );
        assert_eq!(
            "claude-code".parse::<LlmProvider>().unwrap(),
            LlmProvider::ClaudeCode
        );
        assert_eq!(
            "claude".parse::<LlmProvider>().unwrap(),
            LlmProvider::ClaudeCode
        );
        assert_eq!(
            "gemini-cli".parse::<LlmProvider>().unwrap(),
            LlmProvider::GeminiCli
        );
        assert_eq!(
            "gemini".parse::<LlmProvider>().unwrap(),
            LlmProvider::GeminiCli
        );
        assert!("invalid".parse::<LlmProvider>().is_err());
    }

    #[test]
    fn test_llm_provider_display() {
        assert_eq!(LlmProvider::GoogleAi.to_string(), "google-ai");
        assert_eq!(LlmProvider::ClaudeCode.to_string(), "claude-code");
        assert_eq!(LlmProvider::GeminiCli.to_string(), "gemini-cli");
    }

    #[test]
    fn test_generate_request_builder() {
        let request = GenerateRequest::new("test prompt".to_string())
            .with_max_tokens(1024)
            .with_temperature(0.7);

        assert_eq!(request.prompt, "test prompt");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_llm_config_builders() {
        let config = LlmConfig::google_ai("test-key".to_string())
            .with_model("gemini-2.5-flash".to_string())
            .with_timeout(600);

        assert_eq!(config.provider, LlmProvider::GoogleAi);
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, Some("gemini-2.5-flash".to_string()));
        assert_eq!(config.timeout_secs, 600);
    }
}
