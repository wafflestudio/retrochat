use reqwest::{Client, Response};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use super::errors::{GoogleAiError, RetryError};
use super::models::{GenerateContentRequest, GenerateContentResponse, GenerationConfig};
use super::retry::{with_retry, RetryConfig};
use crate::env::apis as env_vars;

#[derive(Debug, Clone)]
pub struct GoogleAiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
    pub max_retries: usize,
}

impl Default for GoogleAiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var(env_vars::GOOGLE_AI_API_KEY).unwrap_or_default(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model: "gemini-2.5-flash-lite".to_string(),
            timeout: Duration::from_secs(300),
            max_retries: 3,
        }
    }
}

impl GoogleAiConfig {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn validate(&self) -> Result<(), GoogleAiError> {
        if self.api_key.is_empty() {
            return Err(GoogleAiError::ConfigurationError {
                message: "Google AI API key is required".to_string(),
            });
        }

        if self.base_url.is_empty() {
            return Err(GoogleAiError::ConfigurationError {
                message: "Base URL cannot be empty".to_string(),
            });
        }

        if self.model.is_empty() {
            return Err(GoogleAiError::ConfigurationError {
                message: "Model name cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct GoogleAiClient {
    config: GoogleAiConfig,
    client: Client,
    rate_limiter: Arc<Semaphore>,
}

impl GoogleAiClient {
    pub fn new(config: GoogleAiConfig) -> Result<Self, GoogleAiError> {
        config.validate()?;

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| GoogleAiError::ConfigurationError {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        // Rate limiter - allows 15 requests per minute by default
        let rate_limiter = Arc::new(Semaphore::new(15));

        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }

    pub async fn generate_content(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, GoogleAiError> {
        let retry_config =
            RetryConfig::new(self.config.max_retries).with_total_timeout(self.config.timeout);

        with_retry(retry_config, || self.generate_content_once(request.clone()))
            .await
            .map_err(|retry_error| match retry_error {
                RetryError::NonRetryable { source } => source,
                RetryError::MaxAttemptsExceeded => GoogleAiError::RateLimitExceeded {
                    message: "Maximum retry attempts exceeded".to_string(),
                },
                RetryError::TimeoutExceeded => GoogleAiError::Timeout {
                    timeout_ms: self.config.timeout.as_millis() as u64,
                },
            })
    }

    async fn generate_content_once(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, GoogleAiError> {
        // Acquire rate limit permit
        let _permit =
            self.rate_limiter
                .acquire()
                .await
                .map_err(|_| GoogleAiError::RateLimitExceeded {
                    message: "Rate limiter closed".to_string(),
                })?;

        let url = format!(
            "{}/models/{}:generateContent",
            self.config.base_url, self.config.model
        );

        let response = timeout(
            self.config.timeout,
            self.client
                .post(&url)
                .header("x-goog-api-key", &self.config.api_key)
                .header("Content-Type", "application/json")
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| GoogleAiError::Timeout {
            timeout_ms: self.config.timeout.as_millis() as u64,
        })?
        .map_err(GoogleAiError::from_reqwest_error)?;

        self.handle_response(response).await
    }

    async fn handle_response(
        &self,
        response: Response,
    ) -> Result<GenerateContentResponse, GoogleAiError> {
        let status = response.status();

        if status.is_success() {
            let response_text = response
                .text()
                .await
                .map_err(GoogleAiError::from_reqwest_error)?;

            let parsed_response: GenerateContentResponse = serde_json::from_str(&response_text)
                .map_err(|e| GoogleAiError::ParseError {
                    message: format!("Failed to parse response: {e}"),
                })?;

            parsed_response
                .validate()
                .map_err(|e| GoogleAiError::InvalidResponse { message: e })?;

            Ok(parsed_response)
        } else {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            Err(GoogleAiError::from_status_and_body(status, &error_body))
        }
    }

    pub fn build_analysis_request(&self, chat_data: &str) -> GenerateContentRequest {
        let prompt = self.build_analysis_prompt();
        let full_content = format!("{prompt}\n\nChat Session:\n{chat_data}");

        GenerateContentRequest::new(full_content)
            .with_generation_config(GenerationConfig::default())
    }

    fn build_analysis_prompt(&self) -> String {
        r#"Analyze this chat session between a user and an AI coding assistant. Focus on the user's communication patterns, question quality, and interaction effectiveness.

Evaluate the following aspects:
1. Communication Clarity: How clearly does the user express their needs and problems?
2. Question Quality: Are questions specific, well-structured, and provide sufficient context?
3. Follow-up Effectiveness: How well does the user iterate and build on AI responses?
4. Task Breakdown: Does the user effectively break down complex problems?
5. Collaboration Style: How effectively does the user collaborate with the AI?

Provide:
- Overall assessment (1-10 scale for each aspect)
- Specific examples of strengths and areas for improvement
- Actionable recommendations for better AI collaboration"#.to_string()
    }

    pub async fn analyze_session(&self, session_data: &str) -> Result<String, GoogleAiError> {
        let request = self.build_analysis_request(session_data);
        let response = self.generate_content(request).await?;

        response
            .extract_text()
            .ok_or_else(|| GoogleAiError::InvalidResponse {
                message: "No text content in response".to_string(),
            })
    }

    pub async fn analyze(
        &self,
        analysis_request: super::models::AnalysisRequest,
    ) -> Result<super::models::AnalysisResponse, GoogleAiError> {
        // Convert AnalysisRequest to GenerateContentRequest
        let mut generation_config = GenerationConfig::default();
        if let Some(temp) = analysis_request.temperature {
            generation_config.temperature = Some(temp);
        }
        if let Some(max_tokens) = analysis_request.max_tokens {
            generation_config.max_output_tokens = Some(max_tokens);
        }

        let request = GenerateContentRequest::new(analysis_request.prompt)
            .with_generation_config(generation_config);

        let response = self.generate_content(request).await?;

        let text = response
            .extract_text()
            .ok_or_else(|| GoogleAiError::InvalidResponse {
                message: "No text content in response".to_string(),
            })?;

        Ok(super::models::AnalysisResponse {
            text,
            token_usage: response.get_token_usage(),
            model_used: Some(self.config.model.clone()),
            finish_reason: response.get_finish_reason(),
        })
    }

    pub fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple token estimation - roughly 4 characters per token
        (text.len() / 4).max(1) as u32
    }

    pub fn config(&self) -> &GoogleAiConfig {
        &self.config
    }

    pub async fn test_connection(&self) -> Result<(), GoogleAiError> {
        let test_request = GenerateContentRequest::new("Test connection".to_string());
        self.generate_content_once(test_request).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid_config = GoogleAiConfig::new("valid_key".to_string());
        assert!(valid_config.validate().is_ok());

        let invalid_config = GoogleAiConfig::new("".to_string());
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_analysis_prompt_generation() {
        let config = GoogleAiConfig::new("test_key".to_string());
        let client = GoogleAiClient::new(config).unwrap();

        let prompt = client.build_analysis_prompt();
        assert!(prompt.contains("communication patterns"));
        assert!(prompt.contains("Communication Clarity"));
    }

    #[test]
    fn test_token_estimation() {
        let config = GoogleAiConfig::new("test_key".to_string());
        let client = GoogleAiClient::new(config).unwrap();

        assert_eq!(client.estimate_tokens("test"), 1);
        assert_eq!(client.estimate_tokens("this is a longer test string"), 7);
    }

    #[test]
    fn test_build_analysis_request() {
        let config = GoogleAiConfig::new("test_key".to_string());
        let client = GoogleAiClient::new(config).unwrap();

        let request = client.build_analysis_request("Test chat data");

        assert!(!request.contents.is_empty());
        assert!(request.generation_config.is_some());
    }
}
