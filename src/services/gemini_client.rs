use crate::models::AnalysisMetadata;
use anyhow::{anyhow, Result};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Instant;
use tracing::{debug, error, info, warn};

const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
const MAX_TOKENS: u32 = 8192;
const REQUEST_TIMEOUT_SECONDS: u64 = 120;

#[derive(Debug, Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
    #[serde(rename = "safetySettings")]
    safety_settings: Vec<SafetySetting>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "topK")]
    top_k: u32,
}

#[derive(Debug, Serialize)]
struct SafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    #[serde(rename = "modelVersion")]
    #[allow(dead_code)]
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
    #[serde(rename = "finishReason")]
    #[allow(dead_code)]
    finish_reason: Option<String>,
    #[allow(dead_code)]
    index: Option<u32>,
    #[serde(rename = "safetyRatings")]
    #[allow(dead_code)]
    safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
    #[allow(dead_code)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    #[allow(dead_code)]
    total_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct SafetyRating {
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    probability: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    code: u32,
    message: String,
    status: String,
}

impl GeminiClient {
    /// Create a new Gemini client with API key from environment
    pub fn new() -> Result<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY environment variable not set"))?;

        Self::with_config(api_key, DEFAULT_MODEL.to_string())
    }

    /// Create a new Gemini client with custom configuration
    pub fn with_config(api_key: String, model: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .user_agent("retrochat/1.0")
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {e}"))?;

        Ok(Self {
            client,
            api_key,
            model,
            base_url: GEMINI_API_BASE_URL.to_string(),
        })
    }

    /// Generate content using the Gemini API
    pub async fn generate_content(&self, prompt: &str) -> Result<(String, AnalysisMetadata)> {
        let start_time = Instant::now();

        debug!(
            "Generating content with prompt length: {} chars",
            prompt.len()
        );

        let request = self.build_request(prompt)?;
        let url = format!("{}/models/{}:generateContent", self.base_url, self.model);

        debug!("Making request to: {}", url);

        let response = self
            .client
            .post(&url)
            .query(&[("key", &self.api_key)])
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Gemini API: {e}"))?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {e}"))?;

        debug!("Received response: {} chars", response_text.len());

        let api_response: GenerateContentResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse API response: {e}"))?;

        self.extract_content_and_metadata(api_response, execution_time_ms, Some(response_text))
    }

    /// Generate content with custom temperature and other parameters
    pub async fn generate_content_with_config(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<(String, AnalysisMetadata)> {
        let start_time = Instant::now();

        let request = self.build_request_with_config(prompt, temperature, max_tokens)?;
        let url = format!("{}/models/{}:generateContent", self.base_url, self.model);

        let response = self
            .client
            .post(&url)
            .query(&[("key", &self.api_key)])
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Gemini API: {e}"))?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {e}"))?;

        let api_response: GenerateContentResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse API response: {e}"))?;

        self.extract_content_and_metadata(api_response, execution_time_ms, Some(response_text))
    }

    /// Test the API connection
    pub async fn test_connection(&self) -> Result<()> {
        info!("Testing Gemini API connection...");

        let test_prompt =
            "Hello, please respond with 'Connection successful' to confirm the API is working.";
        let (response, metadata) = self.generate_content(test_prompt).await?;

        if response.to_lowercase().contains("connection successful") {
            info!(
                "API connection test successful. Response time: {}ms, Tokens: {}",
                metadata.execution_time_ms, metadata.total_tokens
            );
            Ok(())
        } else {
            warn!(
                "API connection test received unexpected response: {}",
                response
            );
            Err(anyhow!("API connection test failed - unexpected response"))
        }
    }

    /// Get the current model being used
    pub fn get_model(&self) -> &str {
        &self.model
    }

    /// Get rate limit information (estimated)
    pub fn get_rate_limit_info(&self) -> RateLimitInfo {
        // Gemini 2.5 Flash Lite rate limits (as of 2025)
        match self.model.as_str() {
            "gemini-2.5-flash-lite" => RateLimitInfo {
                requests_per_minute: 1500,
                tokens_per_minute: 1_000_000,
                requests_per_day: 50_000,
            },
            "gemini-2.5-flash" => RateLimitInfo {
                requests_per_minute: 1000,
                tokens_per_minute: 4_000_000,
                requests_per_day: 50_000,
            },
            "gemini-2.5-pro" => RateLimitInfo {
                requests_per_minute: 360,
                tokens_per_minute: 4_000_000,
                requests_per_day: 50_000,
            },
            _ => RateLimitInfo {
                requests_per_minute: 1500,
                tokens_per_minute: 1_000_000,
                requests_per_day: 50_000,
            },
        }
    }

    fn build_request(&self, prompt: &str) -> Result<GenerateContentRequest> {
        self.build_request_with_config(prompt, 0.7, None)
    }

    fn build_request_with_config(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<GenerateContentRequest> {
        if prompt.is_empty() {
            return Err(anyhow!("Prompt cannot be empty"));
        }

        if prompt.len() > 100_000 {
            return Err(anyhow!("Prompt too long (max 100,000 characters)"));
        }

        let max_output_tokens = max_tokens.unwrap_or(MAX_TOKENS);

        Ok(GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                max_output_tokens,
                temperature: temperature.clamp(0.0, 2.0),
                top_p: 0.95,
                top_k: 40,
            },
            safety_settings: vec![
                SafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                SafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                SafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                SafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ],
        })
    }

    async fn handle_error_response(
        &self,
        response: Response,
    ) -> Result<(String, AnalysisMetadata)> {
        let status = response.status();
        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());

        error!(
            "Gemini API error - Status: {}, Response: {}",
            status, response_text
        );

        // Try to parse error response
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
            let error_msg = format!(
                "Gemini API error ({}): {} - {}",
                error_response.error.code,
                error_response.error.status,
                error_response.error.message
            );

            match status.as_u16() {
                400 => Err(anyhow!("Bad request: {error_msg}")),
                401 => Err(anyhow!(
                    "Authentication failed - check GEMINI_API_KEY: {error_msg}"
                )),
                403 => Err(anyhow!(
                    "Permission denied - check API key permissions: {error_msg}"
                )),
                429 => Err(anyhow!(
                    "Rate limit exceeded - please retry later: {error_msg}"
                )),
                500..=599 => Err(anyhow!("Gemini API server error: {error_msg}")),
                _ => Err(anyhow!("Unexpected API error: {error_msg}")),
            }
        } else {
            Err(anyhow!("HTTP {status} - {response_text}"))
        }
    }

    fn extract_content_and_metadata(
        &self,
        response: GenerateContentResponse,
        execution_time_ms: u64,
        raw_response: Option<String>,
    ) -> Result<(String, AnalysisMetadata)> {
        // Extract content
        let content = response
            .candidates
            .first()
            .ok_or_else(|| anyhow!("No candidates in API response"))?
            .content
            .parts
            .first()
            .ok_or_else(|| anyhow!("No content parts in API response"))?
            .text
            .clone();

        if content.is_empty() {
            return Err(anyhow!("Empty content in API response"));
        }

        // Extract token usage
        let usage = response.usage_metadata.unwrap_or(UsageMetadata {
            prompt_token_count: 0,
            candidates_token_count: 0,
            total_token_count: 0,
        });

        // Create metadata
        let metadata = if let Some(raw_response) = raw_response {
            AnalysisMetadata::with_api_metadata(
                self.model.clone(),
                usage.prompt_token_count,
                usage.candidates_token_count,
                execution_time_ms,
                raw_response,
            )
        } else {
            AnalysisMetadata::new(
                self.model.clone(),
                usage.prompt_token_count,
                usage.candidates_token_count,
                execution_time_ms,
            )
        };

        info!(
            "Generated content: {} chars, {} tokens in {}ms",
            content.len(),
            metadata.total_tokens,
            metadata.execution_time_ms
        );

        Ok((content, metadata))
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub requests_per_day: u32,
}

impl Default for GeminiClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default GeminiClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_test_api_key() -> String {
        env::var("GEMINI_API_KEY").unwrap_or_else(|_| "test_api_key".to_string())
    }

    #[test]
    fn test_client_creation() {
        let client =
            GeminiClient::with_config(get_test_api_key(), "gemini-2.5-flash-lite".to_string());
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.get_model(), "gemini-2.5-flash-lite");
    }

    #[test]
    fn test_request_building() {
        let client =
            GeminiClient::with_config(get_test_api_key(), "gemini-2.5-flash-lite".to_string())
                .unwrap();

        // Valid request
        let request = client.build_request("Test prompt");
        assert!(request.is_ok());

        // Empty prompt
        let request = client.build_request("");
        assert!(request.is_err());

        // Very long prompt
        let long_prompt = "a".repeat(200_000);
        let request = client.build_request(&long_prompt);
        assert!(request.is_err());
    }

    #[test]
    fn test_rate_limit_info() {
        let client =
            GeminiClient::with_config(get_test_api_key(), "gemini-2.5-flash-lite".to_string())
                .unwrap();
        let rate_limit = client.get_rate_limit_info();

        assert!(rate_limit.requests_per_minute > 0);
        assert!(rate_limit.tokens_per_minute > 0);
        assert!(rate_limit.requests_per_day > 0);
    }

    #[test]
    fn test_custom_config() {
        let client =
            GeminiClient::with_config(get_test_api_key(), "gemini-2.5-pro".to_string()).unwrap();
        assert_eq!(client.get_model(), "gemini-2.5-pro");

        let rate_limit = client.get_rate_limit_info();
        // Pro model has different rate limits
        assert!(rate_limit.requests_per_minute <= 1000);
    }

    #[test]
    fn test_request_with_config() {
        let client =
            GeminiClient::with_config(get_test_api_key(), "gemini-2.5-flash-lite".to_string())
                .unwrap();

        let request = client.build_request_with_config("Test", 1.5, Some(1000));
        assert!(request.is_ok());

        let req = request.unwrap();
        assert_eq!(req.generation_config.temperature, 1.5);
        assert_eq!(req.generation_config.max_output_tokens, 1000);

        // Test temperature clamping
        let request = client.build_request_with_config("Test", 3.0, None);
        let req = request.unwrap();
        assert_eq!(req.generation_config.temperature, 2.0); // Should be clamped to 2.0
    }

    // Integration tests (require GEMINI_API_KEY to be set)
    #[tokio::test]
    #[ignore] // Use 'cargo test -- --ignored' to run
    async fn test_api_connection() {
        if env::var("GEMINI_API_KEY").is_err() {
            println!("Skipping API test - GEMINI_API_KEY not set");
            return;
        }

        let client = GeminiClient::new().unwrap();
        let result = client.test_connection().await;

        match result {
            Ok(()) => println!("API connection test passed"),
            Err(e) => println!("API connection test failed: {e}"),
        }
    }

    #[tokio::test]
    #[ignore] // Use 'cargo test -- --ignored' to run
    async fn test_content_generation() {
        if env::var("GEMINI_API_KEY").is_err() {
            println!("Skipping API test - GEMINI_API_KEY not set");
            return;
        }

        let client = GeminiClient::new().unwrap();
        let prompt = "Write a brief summary of what machine learning is in one sentence.";

        let result = client.generate_content(prompt).await;

        match result {
            Ok((content, metadata)) => {
                println!("Generated content: {content}");
                println!("Metadata: {metadata:?}");
                assert!(!content.is_empty());
                assert!(metadata.total_tokens > 0);
            }
            Err(e) => {
                println!("Content generation failed: {e}");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Use 'cargo test -- --ignored' to run
    async fn test_content_generation_with_config() {
        if env::var("GEMINI_API_KEY").is_err() {
            println!("Skipping API test - GEMINI_API_KEY not set");
            return;
        }

        let client = GeminiClient::new().unwrap();
        let prompt = "Generate a creative story about a robot in exactly 50 words.";

        let result = client
            .generate_content_with_config(prompt, 1.2, Some(100))
            .await;

        match result {
            Ok((content, metadata)) => {
                println!("Generated creative content: {content}");
                println!("Metadata: {metadata:?}");
                assert!(!content.is_empty());
                assert!(metadata.total_tokens > 0);
            }
            Err(e) => {
                println!("Creative content generation failed: {e}");
            }
        }
    }
}
