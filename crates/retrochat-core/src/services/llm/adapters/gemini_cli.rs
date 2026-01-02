//! Gemini CLI adapter implementing LlmClient trait
//!
//! This adapter invokes the Gemini CLI (`gemini`) as a subprocess
//! with extensions disabled for text-only generation.

use async_trait::async_trait;
use serde::Deserialize;

use crate::env::llm as env_llm;

use super::super::errors::LlmError;
use super::super::subprocess::{check_cli_available, run_cli_command};
use super::super::traits::LlmClient;
use super::super::types::{GenerateRequest, GenerateResponse, LlmConfig, TokenUsage};

/// Gemini CLI output format (when using --output-format json)
#[derive(Debug, Deserialize)]
struct GeminiCliOutput {
    #[serde(default)]
    response: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stats: Option<GeminiStats>,
}

#[derive(Debug, Deserialize)]
struct GeminiStats {
    #[serde(default)]
    models: Option<serde_json::Value>,
}

/// Gemini CLI client using subprocess invocation
pub struct GeminiCliClient {
    cli_path: String,
    model: String,
    timeout_secs: u64,
}

impl GeminiCliClient {
    /// Create a new Gemini CLI client from LlmConfig
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let cli_path = config
            .cli_path
            .or_else(|| std::env::var(env_llm::GEMINI_CLI_PATH).ok())
            .unwrap_or_else(|| "gemini".to_string());

        let model = config
            .model
            .or_else(|| std::env::var(env_llm::RETROCHAT_LLM_MODEL).ok())
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());

        Ok(Self {
            cli_path,
            model,
            timeout_secs: config.timeout_secs,
        })
    }

    fn parse_output(&self, stdout: &str, stderr: &str) -> Result<GenerateResponse, LlmError> {
        // Try to parse as JSON first
        if let Ok(output) = serde_json::from_str::<GeminiCliOutput>(stdout) {
            if let Some(error) = output.error {
                return Err(LlmError::CliExecutionError { message: error });
            }

            let text =
                output
                    .response
                    .or(output.text)
                    .ok_or_else(|| LlmError::InvalidResponse {
                        message: "No response or text in Gemini CLI output".to_string(),
                    })?;

            // Try to extract token usage from stats
            let token_usage = self.extract_token_usage(&output.stats);

            return Ok(GenerateResponse {
                text,
                token_usage,
                model_used: output.model.or_else(|| Some(self.model.clone())),
                finish_reason: Some("stop".to_string()),
                metadata: None,
            });
        }

        // If not JSON, treat stdout as plain text response
        if !stdout.trim().is_empty() {
            return Ok(GenerateResponse {
                text: stdout.trim().to_string(),
                token_usage: None,
                model_used: Some(self.model.clone()),
                finish_reason: Some("stop".to_string()),
                metadata: None,
            });
        }

        // Check stderr for errors
        if !stderr.trim().is_empty() {
            return Err(LlmError::CliExecutionError {
                message: stderr.trim().to_string(),
            });
        }

        Err(LlmError::InvalidResponse {
            message: "Empty response from Gemini CLI".to_string(),
        })
    }

    fn extract_token_usage(&self, stats: &Option<GeminiStats>) -> Option<TokenUsage> {
        let stats = stats.as_ref()?;
        let models = stats.models.as_ref()?;

        // Try to find token info in the stats structure
        // The structure might be: {"models": {"gemini-2.5-pro": {"tokens": {"prompt": N, "candidates": M}}}}
        if let Some(model_stats) = models.as_object() {
            for (_, model_data) in model_stats {
                if let Some(tokens) = model_data.get("tokens") {
                    let input = tokens
                        .get("prompt")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    let output = tokens
                        .get("candidates")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    let total = tokens
                        .get("total")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);

                    return Some(TokenUsage {
                        input_tokens: input,
                        output_tokens: output,
                        total_tokens: total.or_else(|| match (input, output) {
                            (Some(i), Some(o)) => Some(i + o),
                            _ => None,
                        }),
                    });
                }
            }
        }

        None
    }
}

#[async_trait]
impl LlmClient for GeminiCliClient {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, LlmError> {
        // Build command arguments
        // Usage: gemini "prompt" --output-format json -e none
        let args = vec![
            &request.prompt as &str,
            "--output-format",
            "json",
            "-e",
            "none", // Disable extensions
        ];

        let result = run_cli_command(&self.cli_path, &args, self.timeout_secs).await?;

        if result.exit_code != 0 {
            return Err(LlmError::CliExecutionError {
                message: format!(
                    "Gemini CLI exited with code {}: {}",
                    result.exit_code,
                    result.stderr.trim()
                ),
            });
        }

        self.parse_output(&result.stdout, &result.stderr)
    }

    fn provider_name(&self) -> &'static str {
        "gemini-cli"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        if !check_cli_available(&self.cli_path).await {
            return Err(LlmError::CliBinaryNotFound {
                path: self.cli_path.clone(),
            });
        }

        // Try a simple command to verify setup
        let result = run_cli_command(
            &self.cli_path,
            &["Say 'ok'", "--output-format", "json", "-e", "none"],
            30,
        )
        .await?;

        if result.exit_code != 0 {
            return Err(LlmError::AuthenticationFailed {
                message: format!(
                    "Gemini CLI not authenticated or configured: {}",
                    result.stderr.trim()
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = LlmConfig::gemini_cli();
        let client = GeminiCliClient::new(config).unwrap();

        assert_eq!(client.provider_name(), "gemini-cli");
        assert!(!client.model_name().is_empty());
    }

    #[test]
    fn test_client_with_custom_path() {
        let config = LlmConfig::gemini_cli().with_cli_path("/custom/path/gemini".to_string());
        let client = GeminiCliClient::new(config).unwrap();

        assert_eq!(client.cli_path, "/custom/path/gemini");
    }

    #[test]
    fn test_parse_json_output() {
        let config = LlmConfig::gemini_cli();
        let client = GeminiCliClient::new(config).unwrap();

        let json = r#"{"response": "Hello, world!", "model": "gemini-2.5-flash"}"#;
        let result = client.parse_output(json, "");

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Hello, world!");
        assert_eq!(response.model_used, Some("gemini-2.5-flash".to_string()));
    }

    #[test]
    fn test_parse_plain_text_output() {
        let config = LlmConfig::gemini_cli();
        let client = GeminiCliClient::new(config).unwrap();

        let result = client.parse_output("Plain text response", "");

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Plain text response");
    }

    #[test]
    fn test_parse_error_output() {
        let config = LlmConfig::gemini_cli();
        let client = GeminiCliClient::new(config).unwrap();

        let json = r#"{"error": "Authentication failed"}"#;
        let result = client.parse_output(json, "");

        assert!(result.is_err());
        if let Err(LlmError::CliExecutionError { message }) = result {
            assert!(message.contains("Authentication failed"));
        } else {
            panic!("Expected CliExecutionError");
        }
    }
}
