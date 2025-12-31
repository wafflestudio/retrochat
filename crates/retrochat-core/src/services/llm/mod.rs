//! LLM Provider abstraction layer
//!
//! This module provides a unified interface for multiple LLM providers:
//! - Google AI API (remote, requires API key)
//! - Claude Code CLI (local subprocess, uses existing browser authentication)
//! - Gemini CLI (local subprocess, uses existing browser authentication)

pub mod claude_code;
pub mod factory;
pub mod gemini_cli;
pub mod google_ai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// Re-exports
pub use claude_code::ClaudeCodeProvider;
pub use factory::{create_provider, list_available_providers, LlmProviderConfig};
pub use gemini_cli::GeminiCliProvider;
pub use google_ai::GoogleAiProvider;

/// Errors that can occur when using an LLM provider
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Content blocked by safety filters: {message}")]
    ContentBlocked { message: String },

    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },

    #[error("Provider not available: {message}")]
    NotAvailable { message: String },

    #[error("Subprocess error: {message}")]
    Subprocess { message: String },

    #[error("Provider error: {message}")]
    ProviderError { message: String },
}

impl LlmError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimit { .. } | LlmError::Timeout { .. } | LlmError::Network { .. }
        )
    }
}

/// Token usage information from LLM response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Number of tokens in the prompt/input
    pub prompt_tokens: Option<u32>,
    /// Number of tokens in the completion/output
    pub completion_tokens: Option<u32>,
    /// Total tokens used
    pub total_tokens: Option<u32>,
}

impl TokenUsage {
    pub fn new(prompt: Option<u32>, completion: Option<u32>, total: Option<u32>) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
        }
    }
}

/// Request to generate content from an LLM
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// The prompt to send to the LLM
    pub prompt: String,
    /// Maximum tokens to generate (optional, provider-specific default)
    pub max_tokens: Option<u32>,
    /// Temperature for response generation (optional, provider-specific default)
    pub temperature: Option<f32>,
    /// System prompt/context (optional)
    pub system_prompt: Option<String>,
}

impl LlmRequest {
    /// Create a new LLM request with just a prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        }
    }

    /// Set the maximum tokens to generate
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature for generation
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set a system prompt
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }
}

/// Response from an LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The generated text content
    pub text: String,
    /// Token usage information (if available)
    pub token_usage: Option<TokenUsage>,
    /// The model that was used
    pub model: Option<String>,
    /// Finish reason (e.g., "stop", "length")
    pub finish_reason: Option<String>,
}

impl LlmResponse {
    /// Create a new LLM response
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            token_usage: None,
            model: None,
            finish_reason: None,
        }
    }

    /// Set token usage information
    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    /// Set the model name
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the finish reason
    pub fn with_finish_reason(mut self, reason: impl Into<String>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }
}

/// Enum representing the available LLM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderType {
    /// Google AI API (Gemini models via API)
    #[default]
    GoogleAi,
    /// Claude Code CLI (local subprocess)
    ClaudeCode,
    /// Gemini CLI (local subprocess)
    GeminiCli,
}

impl LlmProviderType {
    /// Get all available provider types
    pub fn all() -> &'static [LlmProviderType] {
        &[
            LlmProviderType::GoogleAi,
            LlmProviderType::ClaudeCode,
            LlmProviderType::GeminiCli,
        ]
    }
}

impl fmt::Display for LlmProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmProviderType::GoogleAi => write!(f, "google_ai"),
            LlmProviderType::ClaudeCode => write!(f, "claude_code"),
            LlmProviderType::GeminiCli => write!(f, "gemini_cli"),
        }
    }
}

impl std::str::FromStr for LlmProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "google_ai" | "googleai" | "google-ai" | "gemini-api" => Ok(LlmProviderType::GoogleAi),
            "claude_code" | "claudecode" | "claude-code" | "claude" => {
                Ok(LlmProviderType::ClaudeCode)
            }
            "gemini_cli" | "geminicli" | "gemini-cli" | "gemini" => Ok(LlmProviderType::GeminiCli),
            _ => Err(format!(
                "Unknown LLM provider: '{}'. Valid options: google_ai, claude_code, gemini_cli",
                s
            )),
        }
    }
}

/// Trait for LLM providers
///
/// All LLM providers must implement this trait to be used with the analysis system.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> LlmProviderType;

    /// Get the model name being used
    fn model_name(&self) -> &str;

    /// Check if the provider is available and properly configured
    async fn is_available(&self) -> bool;

    /// Generate content from the LLM
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Estimate the number of tokens in a text (rough estimate)
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Default implementation: ~4 characters per token
        (text.len() / 4).max(1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(
            "google_ai".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::GoogleAi
        );
        assert_eq!(
            "claude_code".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::ClaudeCode
        );
        assert_eq!(
            "gemini_cli".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::GeminiCli
        );

        // Test aliases
        assert_eq!(
            "claude".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::ClaudeCode
        );
        assert_eq!(
            "gemini".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::GeminiCli
        );
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(LlmProviderType::GoogleAi.to_string(), "google_ai");
        assert_eq!(LlmProviderType::ClaudeCode.to_string(), "claude_code");
        assert_eq!(LlmProviderType::GeminiCli.to_string(), "gemini_cli");
    }

    #[test]
    fn test_llm_request_builder() {
        let request = LlmRequest::new("Hello, world!")
            .with_max_tokens(100)
            .with_temperature(0.7)
            .with_system_prompt("You are a helpful assistant.");

        assert_eq!(request.prompt, "Hello, world!");
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(
            request.system_prompt,
            Some("You are a helpful assistant.".to_string())
        );
    }

    #[test]
    fn test_llm_response_builder() {
        let response = LlmResponse::new("Generated text")
            .with_model("test-model")
            .with_finish_reason("stop")
            .with_token_usage(TokenUsage::new(Some(10), Some(20), Some(30)));

        assert_eq!(response.text, "Generated text");
        assert_eq!(response.model, Some("test-model".to_string()));
        assert_eq!(response.finish_reason, Some("stop".to_string()));
        assert!(response.token_usage.is_some());
    }

    #[test]
    fn test_llm_error_retryable() {
        assert!(LlmError::RateLimit { message: "".into() }.is_retryable());
        assert!(LlmError::Timeout { timeout_ms: 1000 }.is_retryable());
        assert!(LlmError::Network { message: "".into() }.is_retryable());

        assert!(!LlmError::Authentication { message: "".into() }.is_retryable());
        assert!(!LlmError::Configuration { message: "".into() }.is_retryable());
    }
}
