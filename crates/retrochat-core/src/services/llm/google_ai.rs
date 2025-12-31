//! Google AI API provider adapter
//!
//! This module wraps the existing GoogleAiClient to implement the LlmProvider trait.

use async_trait::async_trait;

use super::{LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse, TokenUsage};
use crate::services::google_ai::{GoogleAiClient, GoogleAiConfig, GoogleAiError};

/// Google AI API provider
///
/// This provider uses the Google AI API (Gemini models) via HTTP requests.
/// Requires a valid API key set via GOOGLE_AI_API_KEY environment variable.
#[derive(Clone)]
pub struct GoogleAiProvider {
    client: GoogleAiClient,
}

impl GoogleAiProvider {
    /// Create a new Google AI provider with the given configuration
    pub fn new(config: GoogleAiConfig) -> Result<Self, LlmError> {
        let client = GoogleAiClient::new(config).map_err(|e| LlmError::Configuration {
            message: e.to_string(),
        })?;
        Ok(Self { client })
    }

    /// Create a new Google AI provider from an API key
    pub fn from_api_key(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = GoogleAiConfig::new(api_key.into());
        Self::new(config)
    }

    /// Create a new Google AI provider from environment variable
    pub fn from_env() -> Result<Self, LlmError> {
        let config = GoogleAiConfig::default();
        if config.api_key.is_empty() {
            return Err(LlmError::Configuration {
                message: "GOOGLE_AI_API_KEY environment variable is not set".to_string(),
            });
        }
        Self::new(config)
    }

    /// Get the underlying client configuration
    pub fn config(&self) -> &GoogleAiConfig {
        self.client.config()
    }

    /// Get the underlying GoogleAiClient for legacy compatibility
    pub fn client(&self) -> &GoogleAiClient {
        &self.client
    }
}

impl From<GoogleAiError> for LlmError {
    fn from(err: GoogleAiError) -> Self {
        match err {
            GoogleAiError::AuthenticationFailed { message } => LlmError::Authentication { message },
            GoogleAiError::RateLimitExceeded { message } => LlmError::RateLimit { message },
            GoogleAiError::Timeout { timeout_ms } => LlmError::Timeout { timeout_ms },
            GoogleAiError::NetworkError { source } => LlmError::Network {
                message: source.to_string(),
            },
            GoogleAiError::ContentBlocked => LlmError::ContentBlocked {
                message: "Content blocked by safety filters".to_string(),
            },
            GoogleAiError::InvalidResponse { message } => LlmError::InvalidResponse { message },
            GoogleAiError::ConfigurationError { message } => LlmError::Configuration { message },
            GoogleAiError::InvalidRequest { message } => LlmError::ProviderError { message },
            GoogleAiError::QuotaExceeded { message } => LlmError::RateLimit { message },
            GoogleAiError::ServerError { message, .. } => LlmError::ProviderError { message },
            GoogleAiError::ParseError { message } => LlmError::InvalidResponse { message },
            GoogleAiError::ServiceUnavailable { message } => LlmError::NotAvailable { message },
        }
    }
}

#[async_trait]
impl LlmProvider for GoogleAiProvider {
    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::GoogleAi
    }

    fn model_name(&self) -> &str {
        &self.client.config().model
    }

    async fn is_available(&self) -> bool {
        // Check if we have an API key configured
        !self.client.config().api_key.is_empty()
    }

    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Build the analysis request
        let analysis_request = crate::services::google_ai::models::AnalysisRequest {
            prompt: request.prompt,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        // Call the Google AI API
        let response = self.client.analytics(analysis_request).await?;

        // Convert to LlmResponse
        let token_usage = response.token_usage.map(|total| TokenUsage {
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: Some(total),
        });

        Ok(LlmResponse {
            text: response.text,
            token_usage,
            model: response.model_used,
            finish_reason: response.finish_reason,
        })
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        self.client.estimate_tokens(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_ai_provider_from_api_key() {
        let provider = GoogleAiProvider::from_api_key("test-key");
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.provider_type(), LlmProviderType::GoogleAi);
        assert_eq!(provider.model_name(), "gemini-2.5-flash-lite");
    }

    #[test]
    fn test_google_ai_provider_empty_key() {
        let config = GoogleAiConfig::new("".to_string());
        let provider = GoogleAiProvider::new(config);
        assert!(provider.is_err());
    }

    #[tokio::test]
    async fn test_is_available() {
        let provider = GoogleAiProvider::from_api_key("test-key").unwrap();
        assert!(provider.is_available().await);
    }

    #[test]
    fn test_error_conversion() {
        let auth_err = GoogleAiError::AuthenticationFailed {
            message: "Invalid key".to_string(),
        };
        let llm_err: LlmError = auth_err.into();
        assert!(matches!(llm_err, LlmError::Authentication { .. }));

        let rate_err = GoogleAiError::RateLimitExceeded {
            message: "Too many requests".to_string(),
        };
        let llm_err: LlmError = rate_err.into();
        assert!(matches!(llm_err, LlmError::RateLimit { .. }));
    }
}
