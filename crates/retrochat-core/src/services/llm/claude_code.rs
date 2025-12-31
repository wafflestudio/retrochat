//! Claude Code CLI provider
//!
//! This module provides an LLM provider that uses the Claude Code CLI
//! as a subprocess, leveraging existing browser authentication.

use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::{LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse};

/// Default timeout for Claude Code CLI requests (5 minutes)
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default model used by Claude Code
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Claude Code CLI provider configuration
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// Path to the claude CLI binary
    pub binary_path: String,
    /// Model to use (if supported by the CLI)
    pub model: String,
    /// Timeout for requests
    pub timeout: Duration,
    /// Maximum tokens to generate (passed to CLI if supported)
    pub max_tokens: Option<u32>,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            binary_path: "claude".to_string(),
            model: DEFAULT_MODEL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_tokens: Some(4096),
        }
    }
}

impl ClaudeCodeConfig {
    /// Create a new configuration with a custom binary path
    pub fn with_binary_path(mut self, path: impl Into<String>) -> Self {
        self.binary_path = path.into();
        self
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

/// Claude Code CLI provider
///
/// This provider executes the Claude Code CLI (`claude`) as a subprocess
/// using the `-p` (print/prompt) flag for headless operation.
///
/// The CLI uses existing browser authentication, so no API key is required.
#[derive(Debug, Clone)]
pub struct ClaudeCodeProvider {
    config: ClaudeCodeConfig,
}

impl ClaudeCodeProvider {
    /// Create a new Claude Code provider with default configuration
    pub fn new() -> Self {
        Self {
            config: ClaudeCodeConfig::default(),
        }
    }

    /// Create a new Claude Code provider with custom configuration
    pub fn with_config(config: ClaudeCodeConfig) -> Self {
        Self { config }
    }

    /// Check if the Claude CLI binary is available on the system
    async fn check_binary_available(&self) -> Result<bool, LlmError> {
        let result = Command::new("which")
            .arg(&self.config.binary_path)
            .output()
            .await;

        match result {
            Ok(output) => Ok(output.status.success()),
            Err(_) => {
                // On Windows, try 'where' instead
                let result = Command::new("where")
                    .arg(&self.config.binary_path)
                    .output()
                    .await;

                match result {
                    Ok(output) => Ok(output.status.success()),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Execute the Claude CLI with the given prompt
    async fn execute_cli(&self, prompt: &str) -> Result<String, LlmError> {
        // Build the command
        // Using `claude -p "prompt"` for headless operation
        let mut cmd = Command::new(&self.config.binary_path);

        // Add the print/prompt flag and the prompt itself
        cmd.arg("-p")
            .arg(prompt)
            // Disable interactive features
            .arg("--verbose")
            // Set output format to plain text
            .arg("--output-format")
            .arg("text");

        // Add max tokens if specified
        if let Some(max_tokens) = self.config.max_tokens {
            cmd.arg("--max-turns").arg(max_tokens.to_string());
        }

        // Set up stdio
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| LlmError::Subprocess {
            message: format!("Failed to spawn Claude CLI process: {}", e),
        })?;

        // Close stdin to signal we're done sending input
        if let Some(mut stdin) = child.stdin.take() {
            stdin.shutdown().await.ok();
        }

        // Wait for the process with timeout
        let output = timeout(self.config.timeout, child.wait_with_output())
            .await
            .map_err(|_| LlmError::Timeout {
                timeout_ms: self.config.timeout.as_millis() as u64,
            })?
            .map_err(|e| LlmError::Subprocess {
                message: format!("Failed to wait for Claude CLI process: {}", e),
            })?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check for common error patterns
            if stderr.contains("not authenticated") || stderr.contains("authentication") {
                return Err(LlmError::Authentication {
                    message: format!(
                        "Claude CLI authentication failed. Please run 'claude' to authenticate. Error: {}",
                        stderr
                    ),
                });
            }

            if stderr.contains("rate limit") || stderr.contains("too many requests") {
                return Err(LlmError::RateLimit {
                    message: stderr.to_string(),
                });
            }

            return Err(LlmError::Subprocess {
                message: format!(
                    "Claude CLI exited with status {}. Stderr: {}. Stdout: {}",
                    output.status, stderr, stdout
                ),
            });
        }

        // Parse the output
        let response_text = String::from_utf8_lossy(&output.stdout).to_string();

        if response_text.trim().is_empty() {
            return Err(LlmError::InvalidResponse {
                message: "Claude CLI returned empty response".to_string(),
            });
        }

        Ok(response_text)
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for ClaudeCodeProvider {
    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::ClaudeCode
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn is_available(&self) -> bool {
        self.check_binary_available().await.unwrap_or_default()
    }

    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Build the full prompt with system context if provided
        let full_prompt = match &request.system_prompt {
            Some(system) => format!("{}\n\n{}", system, request.prompt),
            None => request.prompt,
        };

        // Execute the CLI
        let response_text = self.execute_cli(&full_prompt).await?;

        // Build the response
        // Note: Claude CLI doesn't provide token usage information
        Ok(LlmResponse::new(response_text).with_model(self.config.model.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ClaudeCodeConfig::default();
        assert_eq!(config.binary_path, "claude");
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_config_builder() {
        let config = ClaudeCodeConfig::default()
            .with_binary_path("/custom/path/claude")
            .with_model("claude-3-opus")
            .with_timeout(Duration::from_secs(600))
            .with_max_tokens(8192);

        assert_eq!(config.binary_path, "/custom/path/claude");
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.timeout, Duration::from_secs(600));
        assert_eq!(config.max_tokens, Some(8192));
    }

    #[test]
    fn test_provider_creation() {
        let provider = ClaudeCodeProvider::new();
        assert_eq!(provider.provider_type(), LlmProviderType::ClaudeCode);
        assert_eq!(provider.model_name(), DEFAULT_MODEL);
    }

    #[tokio::test]
    async fn test_is_available_when_not_installed() {
        // Use a binary that definitely doesn't exist
        let config =
            ClaudeCodeConfig::default().with_binary_path("definitely_not_a_real_binary_12345");
        let provider = ClaudeCodeProvider::with_config(config);

        assert!(!provider.is_available().await);
    }
}
