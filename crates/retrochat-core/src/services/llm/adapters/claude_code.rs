//! Claude Code CLI adapter implementing LlmClient trait
//!
//! This adapter invokes the Claude Code CLI (`claude`) as a subprocess
//! with tools disabled for text-only generation.

use async_trait::async_trait;
use serde::Deserialize;

use crate::env::llm as env_llm;

use super::super::errors::LlmError;
use super::super::subprocess::{check_cli_available, run_cli_command, run_cli_command_with_stdin};
use super::super::traits::LlmClient;
use super::super::types::{GenerateRequest, GenerateResponse, LlmConfig, TokenUsage};

/// Claude Code CLI output format (when using --output-format json)
#[derive(Debug, Deserialize)]
struct ClaudeCodeOutput {
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    cost_usd: Option<f64>,
    #[serde(default)]
    is_error: Option<bool>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

/// Claude Code CLI client using subprocess invocation
pub struct ClaudeCodeClient {
    cli_path: String,
    model: String,
    timeout_secs: u64,
}

impl ClaudeCodeClient {
    /// Create a new Claude Code client from LlmConfig
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let cli_path = config
            .cli_path
            .or_else(|| std::env::var(env_llm::CLAUDE_CODE_PATH).ok())
            .unwrap_or_else(|| "claude".to_string());

        let model = config
            .model
            .or_else(|| std::env::var(env_llm::RETROCHAT_LLM_MODEL).ok())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        Ok(Self {
            cli_path,
            model,
            timeout_secs: config.timeout_secs,
        })
    }

    fn parse_output(&self, stdout: &str, stderr: &str) -> Result<GenerateResponse, LlmError> {
        // Try to parse as JSON first
        if let Ok(output) = serde_json::from_str::<ClaudeCodeOutput>(stdout) {
            if output.is_error == Some(true) || output.error.is_some() {
                return Err(LlmError::CliExecutionError {
                    message: output.error.unwrap_or_else(|| "Unknown error".to_string()),
                });
            }

            let text =
                output
                    .result
                    .or(output.content)
                    .ok_or_else(|| LlmError::InvalidResponse {
                        message: "No result or content in Claude Code output".to_string(),
                    })?;

            let token_usage = output.usage.map(|u| TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: match (u.input_tokens, u.output_tokens) {
                    (Some(i), Some(o)) => Some(i + o),
                    _ => None,
                },
            });

            return Ok(GenerateResponse {
                text,
                token_usage,
                model_used: Some(self.model.clone()),
                finish_reason: Some("stop".to_string()),
                metadata: Some(serde_json::json!({
                    "cost_usd": output.cost_usd,
                    "session_id": output.session_id,
                })),
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
            message: "Empty response from Claude Code".to_string(),
        })
    }
}

#[async_trait]
impl LlmClient for ClaudeCodeClient {
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, LlmError> {
        // Build command arguments
        // Usage: echo "prompt" | claude -p --output-format json --tools "" --no-session-persistence --setting-sources user
        // Using stdin instead of command-line argument to:
        // - Avoid OS argument length limits (typically 128KB-2MB)
        // - Handle special characters without escaping issues
        let args = vec![
            "-p", // Print mode, reads prompt from stdin
            "--output-format",
            "json",
            "--allowedTools",
            "",                         // Disable tools for text-only analysis
            "--no-session-persistence", // Don't save to .claude/projects (avoid polluting retrochat imports)
            "--setting-sources",
            "user", // Only load user settings, skip project/local CLAUDE.md files
        ];

        let result =
            run_cli_command_with_stdin(&self.cli_path, &args, &request.prompt, self.timeout_secs)
                .await?;

        if result.exit_code != 0 {
            return Err(LlmError::CliExecutionError {
                message: format!(
                    "Claude Code exited with code {}: {}",
                    result.exit_code,
                    result.stderr.trim()
                ),
            });
        }

        self.parse_output(&result.stdout, &result.stderr)
    }

    fn provider_name(&self) -> &'static str {
        "claude-code"
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

        // Try a simple command to verify authentication
        let result = run_cli_command(
            &self.cli_path,
            &[
                "-p",
                "Say 'ok'",
                "--output-format",
                "json",
                "--allowedTools",
                "",
                "--no-session-persistence",
                "--setting-sources",
                "user",
            ],
            30,
        )
        .await?;

        if result.exit_code != 0 {
            return Err(LlmError::AuthenticationFailed {
                message: format!(
                    "Claude Code CLI not authenticated: {}",
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
        let config = LlmConfig::claude_code();
        let client = ClaudeCodeClient::new(config).unwrap();

        assert_eq!(client.provider_name(), "claude-code");
        assert!(!client.model_name().is_empty());
    }

    #[test]
    fn test_client_with_custom_path() {
        let config = LlmConfig::claude_code().with_cli_path("/custom/path/claude".to_string());
        let client = ClaudeCodeClient::new(config).unwrap();

        assert_eq!(client.cli_path, "/custom/path/claude");
    }

    #[test]
    fn test_parse_json_output() {
        let config = LlmConfig::claude_code();
        let client = ClaudeCodeClient::new(config).unwrap();

        let json = r#"{"result": "Hello, world!", "cost_usd": 0.001}"#;
        let result = client.parse_output(json, "");

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Hello, world!");
    }

    #[test]
    fn test_parse_plain_text_output() {
        let config = LlmConfig::claude_code();
        let client = ClaudeCodeClient::new(config).unwrap();

        let result = client.parse_output("Plain text response", "");

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Plain text response");
    }

    #[test]
    fn test_parse_error_output() {
        let config = LlmConfig::claude_code();
        let client = ClaudeCodeClient::new(config).unwrap();

        let json = r#"{"is_error": true, "error": "Authentication failed"}"#;
        let result = client.parse_output(json, "");

        assert!(result.is_err());
        if let Err(LlmError::CliExecutionError { message }) = result {
            assert!(message.contains("Authentication failed"));
        } else {
            panic!("Expected CliExecutionError");
        }
    }
}
