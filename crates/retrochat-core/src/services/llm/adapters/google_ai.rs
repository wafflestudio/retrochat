//! Google AI adapter implementing LlmClient trait
//!
//! This adapter wraps the existing GoogleAiClient to provide
//! a unified interface for LLM operations.

use async_trait::async_trait;
use std::time::Duration;

use crate::services::google_ai::models::AnalysisRequest as GaiAnalysisRequest;
use crate::services::google_ai::{GoogleAiClient, GoogleAiConfig};

use super::super::errors::LlmError;
use super::super::traits::LlmClient;
use super::super::types::{GenerateRequest, GenerateResponse, LlmConfig, TokenUsage};

/// Adapter that wraps GoogleAiClient to implement LlmClient trait
pub struct GoogleAiAdapter {
    client: GoogleAiClient,
    model_name: String,
}

impl GoogleAiAdapter {
    /// Create a new adapter from LlmConfig
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let api_key = config.api_key.ok_or_else(|| LlmError::ConfigurationError {
            message: "Google AI API key is required".to_string(),
        })?;

        let mut gai_config = GoogleAiConfig::new(api_key);

        if let Some(model) = &config.model {
            gai_config = gai_config.with_model(model.clone());
        }

        gai_config = gai_config
            .with_timeout(Duration::from_secs(config.timeout_secs))
            .with_max_retries(config.max_retries);

        let model_name = gai_config.model.clone();
        let client = GoogleAiClient::new(gai_config).map_err(LlmError::from)?;

        Ok(Self { client, model_name })
    }

    /// Create adapter from existing GoogleAiClient (for backward compatibility)
    pub fn from_client(client: GoogleAiClient) -> Self {
        let model_name = client.config().model.clone();
        Self { client, model_name }
    }
}

#[async_trait]
impl LlmClient for GoogleAiAdapter {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, LlmError> {
        let gai_request = GaiAnalysisRequest {
            prompt: request.prompt,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let response = self
            .client
            .analytics(gai_request)
            .await
            .map_err(LlmError::from)?;

        Ok(GenerateResponse {
            text: response.text,
            token_usage: response.token_usage.map(|total| TokenUsage {
                input_tokens: None,
                output_tokens: None,
                total_tokens: Some(total),
            }),
            model_used: response.model_used,
            finish_reason: response.finish_reason,
            metadata: None,
        })
    }

    fn provider_name(&self) -> &'static str {
        "google-ai"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        self.client.test_connection().await.map_err(LlmError::from)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        self.client.estimate_tokens(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation_fails_without_api_key() {
        let config = LlmConfig::default();
        let result = GoogleAiAdapter::new(config);
        assert!(result.is_err());
        if let Err(LlmError::ConfigurationError { message }) = result {
            assert!(message.contains("API key"));
        } else {
            panic!("Expected ConfigurationError");
        }
    }

    #[test]
    fn test_adapter_from_client() {
        let gai_config = GoogleAiConfig::new("test-key".to_string());
        let client = GoogleAiClient::new(gai_config).unwrap();
        let adapter = GoogleAiAdapter::from_client(client);

        assert_eq!(adapter.provider_name(), "google-ai");
        assert!(!adapter.model_name().is_empty());
    }
}
