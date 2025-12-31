//! Gemini CLI provider
//!
//! This module provides an LLM provider that uses the Gemini CLI
//! as a subprocess, leveraging existing browser authentication.
//!
//! The Gemini CLI offers a generous free tier:
//! - 60 requests per minute
//! - 1000 requests per day
//! - Gemini 2.5 Pro with 1M token context window

use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::{LlmError, LlmProvider, LlmProviderType, LlmRequest, LlmResponse};

/// Default timeout for Gemini CLI requests (5 minutes)
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default model used by Gemini CLI
const DEFAULT_MODEL: &str = "gemini-2.5-pro";

/// Gemini CLI provider configuration
#[derive(Debug, Clone)]
pub struct GeminiCliConfig {
    /// Path to the gemini CLI binary
    pub binary_path: String,
    /// Model to use
    pub model: String,
    /// Timeout for requests
    pub timeout: Duration,
}

impl Default for GeminiCliConfig {
    fn default() -> Self {
        Self {
            binary_path: "gemini".to_string(),
            model: DEFAULT_MODEL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

impl GeminiCliConfig {
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
}

/// Gemini CLI provider
///
/// This provider executes the Gemini CLI (`gemini`) as a subprocess
/// using the `-p` flag for headless/prompt operation.
///
/// The CLI uses existing Google authentication (via browser), so no API key is required.
///
/// Free tier limits:
/// - 60 requests per minute
/// - 1000 requests per day
/// - 1M token context window
#[derive(Debug, Clone)]
pub struct GeminiCliProvider {
    config: GeminiCliConfig,
}

impl GeminiCliProvider {
    /// Create a new Gemini CLI provider with default configuration
    pub fn new() -> Self {
        Self {
            config: GeminiCliConfig::default(),
        }
    }

    /// Create a new Gemini CLI provider with custom configuration
    pub fn with_config(config: GeminiCliConfig) -> Self {
        Self { config }
    }

    /// Check if the Gemini CLI binary is available on the system
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

    /// Execute the Gemini CLI with the given prompt
    async fn execute_cli(&self, prompt: &str) -> Result<String, LlmError> {
        // Build the command
        // Using `gemini -p "prompt"` for headless operation
        let mut cmd = Command::new(&self.config.binary_path);

        // Add the prompt flag and the prompt itself
        cmd.arg("-p").arg(prompt);

        // Optionally specify the model
        if self.config.model != DEFAULT_MODEL {
            cmd.arg("--model").arg(&self.config.model);
        }

        // Set up stdio
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| LlmError::Subprocess {
            message: format!("Failed to spawn Gemini CLI process: {}", e),
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
                message: format!("Failed to wait for Gemini CLI process: {}", e),
            })?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check for common error patterns
            if stderr.contains("not authenticated")
                || stderr.contains("authentication")
                || stderr.contains("login")
            {
                return Err(LlmError::Authentication {
                    message: format!(
                        "Gemini CLI authentication failed. Please run 'gemini' to authenticate. Error: {}",
                        stderr
                    ),
                });
            }

            if stderr.contains("rate limit")
                || stderr.contains("quota")
                || stderr.contains("too many requests")
            {
                return Err(LlmError::RateLimit {
                    message: stderr.to_string(),
                });
            }

            if stderr.contains("blocked") || stderr.contains("safety") {
                return Err(LlmError::ContentBlocked {
                    message: stderr.to_string(),
                });
            }

            return Err(LlmError::Subprocess {
                message: format!(
                    "Gemini CLI exited with status {}. Stderr: {}. Stdout: {}",
                    output.status, stderr, stdout
                ),
            });
        }

        // Parse the output
        let response_text = String::from_utf8_lossy(&output.stdout).to_string();

        if response_text.trim().is_empty() {
            return Err(LlmError::InvalidResponse {
                message: "Gemini CLI returned empty response".to_string(),
            });
        }

        Ok(response_text)
    }
}

impl Default for GeminiCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for GeminiCliProvider {
    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::GeminiCli
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
        // Note: Gemini CLI doesn't provide token usage information in headless mode
        Ok(LlmResponse::new(response_text).with_model(self.config.model.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = GeminiCliConfig::default();
        assert_eq!(config.binary_path, "gemini");
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_config_builder() {
        let config = GeminiCliConfig::default()
            .with_binary_path("/custom/path/gemini")
            .with_model("gemini-1.5-flash")
            .with_timeout(Duration::from_secs(600));

        assert_eq!(config.binary_path, "/custom/path/gemini");
        assert_eq!(config.model, "gemini-1.5-flash");
        assert_eq!(config.timeout, Duration::from_secs(600));
    }

    #[test]
    fn test_provider_creation() {
        let provider = GeminiCliProvider::new();
        assert_eq!(provider.provider_type(), LlmProviderType::GeminiCli);
        assert_eq!(provider.model_name(), DEFAULT_MODEL);
    }

    #[tokio::test]
    async fn test_is_available_when_not_installed() {
        // Use a binary that definitely doesn't exist
        let config =
            GeminiCliConfig::default().with_binary_path("definitely_not_a_real_binary_12345");
        let provider = GeminiCliProvider::with_config(config);

        assert!(!provider.is_available().await);
    }
}
